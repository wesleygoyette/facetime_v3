use std::{collections::HashMap, error::Error, sync::Arc};

use log::{error, info};
use shared::{
    ADD_USER_TO_CLIENT_BYTE, DENY_CALL_BYTE, END_CALL_BYTE, HELLO_FROM_CLIENT_BYTE,
    HELLO_FROM_SERVER_BYTE, REMOVE_USER_FROM_CLIENT_BYTE, REQUEST_CALL_BYTE,
    REQUEST_CALL_STREAM_ID_BYTE, SEND_CALL_STREAM_ID_BYTE, START_CALL_BYTE, TCP_PORT, UDP_PORT,
    USERNAME_ALREADY_TAKEN_BYTE, receive_command_from_stream, send_command_to_stream,
};
use tokio::{
    io::AsyncWriteExt,
    net::{TcpListener, TcpStream, UdpSocket},
    sync::{Mutex, broadcast},
};

#[derive(Debug)]
struct Call {
    usernames_to_sids: HashMap<String, [u8; 4]>,
    sids_requested: u16,
}

pub struct WeSFU {
    tcp_listener: TcpListener,
    udp_socket: UdpSocket,
}

impl WeSFU {
    pub async fn new(
        tcp_addr: String,
        udp_addr: String,
    ) -> Result<WeSFU, Box<dyn Error + Send + Sync>> {
        info!("WeSFU listening on tcp: {}, udp: {}", TCP_PORT, UDP_PORT);
        Ok(Self {
            tcp_listener: TcpListener::bind(tcp_addr).await?,
            udp_socket: UdpSocket::bind(udp_addr).await?,
        })
    }

    pub async fn run(self) -> Result<(), Box<dyn Error + Send + Sync>> {
        let username_to_tcp_command_channel: Arc<
            Mutex<HashMap<String, broadcast::Sender<(u8, Option<String>)>>>,
        > = Arc::new(Mutex::new(HashMap::new()));

        let active_calls: Arc<Mutex<Vec<Call>>> = Arc::new(Mutex::new(Vec::new()));
        let active_calls_for_udp = active_calls.clone();

        tokio::spawn(async move {
            if let Err(e) = udp_loop(self.udp_socket, active_calls_for_udp).await {
                error!("UDP Error: {}", e);
            }
        });

        loop {
            let username_to_tcp_command_channel = username_to_tcp_command_channel.clone();
            let active_calls = active_calls.clone();

            let (mut stream, addr) = self.tcp_listener.accept().await?;
            info!("Opened connection from {}", addr);

            tokio::spawn(async move {
                let current_username: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));

                if let Err(e) = handle_connection(
                    &mut stream,
                    current_username.clone(),
                    username_to_tcp_command_channel.clone(),
                    active_calls.clone(),
                )
                .await
                {
                    error!("Connection error: {}", e);
                }

                if let Some(current_username) = current_username.lock().await.take() {
                    username_to_tcp_command_channel
                        .lock()
                        .await
                        .remove(&current_username);

                    for (username, tcp_command_channel) in
                        username_to_tcp_command_channel.lock().await.iter()
                    {
                        if let Err(e) = tcp_command_channel
                            .send((REMOVE_USER_FROM_CLIENT_BYTE, Some(current_username.clone())))
                        {
                            error!(
                                "Error removing {} from {}: {}",
                                current_username, username, e
                            );
                        }
                    }

                    for call in active_calls.lock().await.iter() {
                        if !call.usernames_to_sids.contains_key(&current_username) {
                            continue;
                        }

                        let call_names: Vec<&String> = call.usernames_to_sids.keys().collect();

                        info!("Call ended between {} and {}", call_names[0], call_names[1]);

                        for username in call.usernames_to_sids.keys() {
                            if let Some(tx) =
                                username_to_tcp_command_channel.lock().await.get(username)
                            {
                                if let Err(e) = tx.send((END_CALL_BYTE, None)) {
                                    error!("Errors end call: {}", e);
                                }
                            }
                        }
                    }

                    active_calls
                        .lock()
                        .await
                        .retain(|call| !call.usernames_to_sids.contains_key(&current_username));

                    info!("{} has disconnected!", current_username);
                }

                info!("Closed connection from {}", addr);
            });
        }
    }
}

async fn udp_loop(
    udp_socket: UdpSocket,
    active_calls: Arc<Mutex<Vec<Call>>>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let mut sids_to_udp_addrs = HashMap::new();

    let mut buf = [0; 4844];

    loop {
        let (n, addr) = udp_socket.recv_from(&mut buf).await?;

        info!("UDP: Recevied {} bytes from {}", n, addr);

        let sid: [u8; 4] = buf[0..4].try_into()?;
        let message = &buf[4..n];

        match sids_to_udp_addrs.get(&sid) {
            Some(_) => {
                let mut other_sid = None;

                let active_calls_guard = active_calls.lock().await;

                for call in active_calls_guard.iter() {
                    if call.usernames_to_sids.values().any(|s| s == &sid) {
                        for other in call.usernames_to_sids.values() {
                            if other != &sid {
                                other_sid = Some(*other);
                                break;
                            }
                        }
                        break;
                    }
                }

                if let Some(other_sid) = other_sid {
                    if let Some(udp_addr) = sids_to_udp_addrs.get(&other_sid) {
                        udp_socket.send_to(message, udp_addr).await?;
                    }
                }
            }
            None => {
                sids_to_udp_addrs.insert(sid, addr);
            }
        }
    }
}

async fn handle_connection(
    stream: &mut TcpStream,
    current_username: Arc<Mutex<Option<String>>>,
    username_to_tcp_command_channel: Arc<
        Mutex<HashMap<String, broadcast::Sender<(u8, Option<String>)>>>,
    >,
    active_calls: Arc<Mutex<Vec<Call>>>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let (tcp_command_channel_tx, mut tcp_command_channel_rx) = broadcast::channel(16);

    loop {
        let tcp_command_channel_tx = tcp_command_channel_tx.clone();

        tokio::select! {

            result = tcp_command_channel_rx.recv() => {

                let (cmd_byte, subject) = result?;

                send_command_to_stream(cmd_byte, subject, stream).await?;
            }

            result = receive_command_from_stream(stream) => {

                match result? {
                    Some((cmd, message)) => match cmd {
                        HELLO_FROM_CLIENT_BYTE => {
                            if let Some(username) = message {

                                if !username_to_tcp_command_channel.lock().await.contains_key(&username) {
                                    *current_username.lock().await = Some(username.clone());
                                    username_to_tcp_command_channel.lock().await.insert(username.clone(), tcp_command_channel_tx);
                                    info!("{} has connected!", username);

                                    send_command_to_stream(HELLO_FROM_SERVER_BYTE, None, stream).await?;

                                    for (user, tcp_command_channel) in username_to_tcp_command_channel.lock().await.iter() {
                                        if *user == username {
                                            continue;
                                        }

                                        if active_calls.lock().await.iter().any(|x| x.usernames_to_sids.contains_key(user)) {

                                            continue;
                                        }

                                        tcp_command_channel.send((ADD_USER_TO_CLIENT_BYTE, Some(username.clone())))?;

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

                                    let username_to_tcp_command_channel_guard = username_to_tcp_command_channel.lock().await;

                                    if let Some(tx) = username_to_tcp_command_channel_guard.get(&username) {

                                        if cmd == START_CALL_BYTE {

                                            let mut usernames_to_sids = HashMap::new();

                                            usernames_to_sids.insert(current_name.clone(), rand::random());
                                            usernames_to_sids.insert(username.clone(), rand::random());

                                            active_calls.lock().await.push(
                                                Call {
                                                    usernames_to_sids: usernames_to_sids,
                                                    sids_requested: 0
                                                }
                                            );

                                            for (user, tx) in username_to_tcp_command_channel_guard.iter() {

                                                if *user == current_name || *user == username {

                                                    continue;
                                                }

                                                tx.send((REMOVE_USER_FROM_CLIENT_BYTE, Some(current_name.clone())))?;
                                                tx.send((REMOVE_USER_FROM_CLIENT_BYTE, Some(username.clone())))?;
                                            }

                                            info!("Call started between {} and {}", current_name, username);
                                        }

                                        tx.send((cmd, Some(current_name.clone())))?;
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

                        REQUEST_CALL_STREAM_ID_BYTE => {
                            if let Some(current_name) = current_username.lock().await.clone() {
                                if let Some(username) = message {
                                    let mut active_calls_guard = active_calls.lock().await;
                                    let mut found_call = None;

                                    for active_call in active_calls_guard.iter_mut() {
                                        if active_call.usernames_to_sids.contains_key(&current_name) {
                                            found_call = Some(active_call);
                                            break;
                                        }
                                    }

                                    match found_call {
                                        Some(call) => {
                                            if let Some(_) = call.usernames_to_sids.get(&username) {
                                                call.sids_requested += 1;
                                                if let Some(current_sid) = call.usernames_to_sids.get(&current_name) {

                                                    let mut message = vec![SEND_CALL_STREAM_ID_BYTE];
                                                    message.extend(current_sid);

                                                    stream.write_all(&message).await?;
                                                    stream.flush().await?;
                                                }
                                                else {
                                                    return Err("Current user SID not found in call".into());
                                                }
                                            }
                                            else {
                                                return Err("Call does not exist".into());
                                            }
                                        }
                                        None => {
                                            return Err("Call does not exist".into());
                                        }
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
