use std::{collections::HashMap, error::Error, sync::Arc, vec};

use log::{error, info};
use shared::{
    receive_command_from_stream, send_command_to_stream, ADD_USER_TO_CLIENT_BYTE, DENY_CALL_BYTE, HELLO_FROM_CLIENT_BYTE, HELLO_FROM_SERVER_BYTE, REMOVE_USER_FROM_CLIENT_BYTE, REQUEST_CALL_BYTE, START_CALL_BYTE, USERNAME_ALREADY_TAKEN_BYTE
};
use tokio::{
    net::{TcpListener, TcpStream},
    sync::{Mutex, broadcast},
};

pub struct WeSFU {
    tcp_listener: TcpListener,
}

impl WeSFU {
    pub async fn new(addr: String) -> Result<WeSFU, Box<dyn Error + Send + Sync>> {
        info!("WeSFU listening on {}", addr);
        Ok(Self {
            tcp_listener: TcpListener::bind(addr).await?,
        })
    }

    pub async fn run(&self) -> Result<(), Box<dyn Error + Send + Sync>> {
        let usernames: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(vec![]));

        let username_to_tcp_command_channel: Arc<
            Mutex<HashMap<String, broadcast::Sender<(u8, String)>>>,
        > = Arc::new(Mutex::new(HashMap::new()));

        loop {
            let usernames = usernames.clone();

            let username_to_tcp_command_channel = username_to_tcp_command_channel.clone();

            let (mut stream, addr) = self.tcp_listener.accept().await?;
            info!("Opened connection from {}", addr);

            tokio::spawn(async move {
                let current_username: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));

                if let Err(e) = handle_connection(
                    &mut stream,
                    current_username.clone(),
                    usernames.clone(),
                    username_to_tcp_command_channel.clone(),
                )
                .await
                {
                    error!("Connection error: {}", e);
                }

                if let Some(current_username) = current_username.lock().await.take() {
                    usernames
                        .lock()
                        .await
                        .retain(|name| name != &current_username);
                    username_to_tcp_command_channel
                        .lock()
                        .await
                        .remove(&current_username);

                    for (username, tcp_command_channel) in
                        username_to_tcp_command_channel.lock().await.iter()
                    {
                        if let Err(e) = tcp_command_channel
                            .send((REMOVE_USER_FROM_CLIENT_BYTE, current_username.clone()))
                        {
                            eprintln!(
                                "Error removing {} from {}: {}",
                                current_username, username, e
                            );
                        }
                    }

                    info!("{} has disconnected!", current_username);
                }

                info!("Closed connection from {}", addr);
            });
        }
    }
}

async fn handle_connection(
    stream: &mut TcpStream,
    current_username: Arc<Mutex<Option<String>>>,
    usernames: Arc<Mutex<Vec<String>>>,
    username_to_tcp_command_channel: Arc<Mutex<HashMap<String, broadcast::Sender<(u8, String)>>>>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let (tcp_command_channel_tx, mut tcp_command_channel_rx) = broadcast::channel(16);

    loop {
        let tcp_command_channel_tx = tcp_command_channel_tx.clone();

        tokio::select! {

            result = tcp_command_channel_rx.recv() => {

                let (cmd_byte, subject) = result?;

                send_command_to_stream(cmd_byte, Some(subject), stream).await?;
            }

            result = receive_command_from_stream(stream) => {

                match result? {
                    Some((cmd, message)) => match cmd {
                        HELLO_FROM_CLIENT_BYTE => {
                            if let Some(username) = message {
                                let mut usernames_guard = usernames.lock().await;

                                if !usernames_guard.contains(&username) {
                                    *current_username.lock().await = Some(username.clone());
                                    username_to_tcp_command_channel.lock().await.insert(username.clone(), tcp_command_channel_tx);
                                    usernames_guard.push(username.clone());
                                    info!("{} has connected!", username);

                                    send_command_to_stream(HELLO_FROM_SERVER_BYTE, None, stream).await?;

                                    for user in usernames_guard.iter() {
                                        if *user == username {
                                            continue;
                                        }

                                        if let Some(tcp_command_channel) = username_to_tcp_command_channel.lock().await.get(user) {

                                            tcp_command_channel.send((ADD_USER_TO_CLIENT_BYTE, username.clone()))?;
                                        }

                                        send_command_to_stream(
                                            ADD_USER_TO_CLIENT_BYTE,
                                            Some(user.to_string()),
                                            stream,
                                        )
                                        .await?;
                                    }
                                } else {

                                    info!("Username: {} was already taken", username);

                                    send_command_to_stream(USERNAME_ALREADY_TAKEN_BYTE, None, stream)
                                        .await?;
                                }
                            } else {
                                return Err("Missing username".into());
                            }
                        }

                        REQUEST_CALL_BYTE | DENY_CALL_BYTE | START_CALL_BYTE => {

                            if let Some(current_name) = current_username.lock().await.clone() {
                                if let Some(username) = message {

                                    if let Some(tx) = username_to_tcp_command_channel.lock().await.get(&username) {

                                        tx.send((cmd, current_name))?;
                                    }
                                    else {
                                        return Err("Invalid username".into());
                                    }
                                }
                                else {
                                    return Err(format!("Missing username {}", cmd).into());
                                }
                            }
                        }
                        _ => {
                            return Err("Invalid command".into());
                        }
                    },
                    None => return Ok(()),
                }
            }
        }
    }
}
