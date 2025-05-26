use crossterm::{
    cursor::MoveTo,
    execute,
    terminal::{Clear, ClearType},
};
use shared::{
    receive_command_from_stream, send_command_to_stream, ADD_USER_TO_CLIENT_BYTE, DENY_CALL_BYTE, HELLO_FROM_CLIENT_BYTE, HELLO_FROM_SERVER_BYTE, REMOVE_USER_FROM_CLIENT_BYTE, REQUEST_CALL_BYTE, START_CALL_BYTE, USERNAME_ALREADY_TAKEN_BYTE
};
use std::{
    error::Error,
    io::{Write, stdout},
    sync::Arc,
};
use tokio::{io::{AsyncBufReadExt}, net::TcpStream, sync::Mutex};

const PROMPT_STRING: &str = "> ";

pub struct Client {
    tcp_stream: TcpStream,
    username: String,
}

impl Client {
    pub async fn new(
        addr: String,
        username: String,
    ) -> Result<Client, Box<dyn Error + Send + Sync>> {
        Ok(Self {
            tcp_stream: TcpStream::connect(addr).await?,
            username: username,
        })
    }

    pub async fn run(&mut self) -> Result<(), Box<dyn Error + Send + Sync>> {
        send_command_to_stream(
            HELLO_FROM_CLIENT_BYTE,
            Some(self.username.clone()),
            &mut self.tcp_stream,
        )
        .await?;

        match receive_command_from_stream(&mut self.tcp_stream).await? {
            Some((cmd, _)) => match cmd {
                HELLO_FROM_SERVER_BYTE => {
                    print_startup_message(self.username.clone())?;
                }
                USERNAME_ALREADY_TAKEN_BYTE => {
                    println!("Username {} already taken!", self.username);
                    return Ok(());
                }
                x => {
                    return Err(format!("Invalid Response from server: {}", x).into());
                }
            },
            None => return Ok(()),
        }

        let raw_stdin = tokio::io::stdin();
        let mut lines = tokio::io::BufReader::new(raw_stdin).lines();

        let available_users: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
        let requesting_call_recipient = Arc::new(Mutex::new(None));

        print!("{}", PROMPT_STRING);
        stdout().flush()?;

        loop {
            tokio::select! {

                result = lines.next_line(), if requesting_call_recipient.lock().await.is_none() => {

                    match result? {
                        Some(text) => {
                            let trimmed = text.trim();

                            match trimmed {
                                "l" => {

                                    let available_users_guard = available_users.lock().await;

                                    if available_users_guard.is_empty() {
                                        println!("No available users");
                                    } else {
                                        println!("Available users:");
                                        for user in available_users_guard.iter() {
                                            println!("  * {}", user);
                                        }
                                    }
                                }
                                "c" => {

                                    println!("Usage: c <username>");
                                }
                                s if s.starts_with("c ") => {

                                    if let Some(username) = s.split_whitespace().nth(1) {

                                        if available_users.lock().await.contains(&username.to_string()) {

                                            println!("Calling {}...", username);
                                            send_command_to_stream(REQUEST_CALL_BYTE, Some(username.to_string()), &mut self.tcp_stream).await?;
                                            *requesting_call_recipient.lock().await = Some(username.to_string());
                                            continue;
                                        }
                                        else {

                                            if username != self.username {

                                                println!("{} is not available.", username);
                                            }
                                            else {

                                                println!("You can't call yourself. Idiot.");
                                            }
                                        }
                                    }
                                    else {

                                        println!("Usage: c <username>");
                                    }
                                }
                                "q" => {
                                    println!("Quitting...");
                                    break;
                                }
                                _ => {
                                    println!("Unknown command");
                                }
                            }
                        }
                        None => {
                            eprintln!("No input");
                        }
                    }

                    print!("{}", PROMPT_STRING);
                    stdout().flush()?;
                }

                result = receive_command_from_stream(&mut self.tcp_stream) => {

                    match result? {
                        Some((cmd, message)) => {

                            match handle_command(cmd, message, available_users.clone(), &mut lines, requesting_call_recipient.clone(), &mut self.tcp_stream).await {
                                Ok(Some(())) => continue,
                                Ok(None) => break,
                                Err(e) => {

                                    eprintln!("Error handling command: {}", e);
                                }
                            }
                        },
                        None => return Ok(()),
                    }
                }
            }
        }

        println!("Connecting...");

        return Ok(());
    }
}

async fn handle_command(
    cmd: u8,
    message: Option<String>,
    available_users: Arc<Mutex<Vec<String>>>,
    lines: &mut tokio::io::Lines<tokio::io::BufReader<tokio::io::Stdin>>,
    requesting_call_recipient: Arc<Mutex<Option<String>>>,
    stream: &mut TcpStream,
) -> Result<Option<()>, Box<dyn Error + Send + Sync>> {
    match cmd {
        ADD_USER_TO_CLIENT_BYTE => match message {
            Some(username) => {
                available_users.lock().await.push(username.clone());
            }
            None => {
                return Err("Invalid data".into());
            }
        }
        REMOVE_USER_FROM_CLIENT_BYTE => match message {
            Some(username) => {
                available_users.lock().await.retain(|u| *u != username);
            }
            None => {
                return Err("Invalid data".into());
            }
        }
        REQUEST_CALL_BYTE => match message {

            Some(username) => {
                println!("\nIncoming call from {}", username);

                loop {

                    print!("Would you like to accept? (y/n): ");
                    stdout().flush()?;

                    if let Some(line) = lines.next_line().await? {
                        match line.trim().to_lowercase().as_str() {
                            "yes" | "y" => {
                                
                                send_command_to_stream(START_CALL_BYTE, Some(username), stream).await?;
                                return Ok(None);
                            },
                            "no" | "n" => {

                                send_command_to_stream(DENY_CALL_BYTE, Some(username), stream).await?;
                                println!("You answered NO.");

                                print!("{}", PROMPT_STRING);
                                stdout().flush()?;
                                break;
                            },
                            _ => println!("Invalid response."),
                        }
                    } else {
                        println!("No input received.");
                    }
                }
            }
            None => {
                return Err("Invalid data".into());
            }
        }

        DENY_CALL_BYTE => {

            if let Some(username) = message {

                if let Some(requesting_call_recipient) = requesting_call_recipient.lock().await.take() {

                    if username == requesting_call_recipient {

                        println!("{} denied the call.", username);
                        print!("{}", PROMPT_STRING);
                        stdout().flush()?;
                    }
                }
            }
            else {
                return Err("Invalid data".into());
            }
        }

        START_CALL_BYTE => {

            return Ok(None);
        }

        _ => {
            return Err("Unknown command".into());
        }
    }

    return Ok(Some(()));
}

fn print_startup_message(username: String) -> Result<(), Box<dyn Error + Send + Sync>> {
    execute!(stdout(), Clear(ClearType::All), MoveTo(0, 0))?;
    stdout().flush()?;

    println!("Connected as: {}", username);
    println!();
    println!("Commands available:");
    println!("  l - List all active users");
    println!("  c - Connect to a user");
    println!("  q - Quit the program");
    println!();

    return Ok(());
}