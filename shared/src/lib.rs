use std::{error::Error, str::from_utf8};

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};

pub const TCP_PORT: u16 = 8080;
pub const UDP_PORT: u16 = 8081;
pub const HELLO_FROM_CLIENT_BYTE: u8 = 69;
pub const HELLO_FROM_SERVER_BYTE: u8 = 70;
pub const USERNAME_ALREADY_TAKEN_BYTE: u8 = 71;
pub const ADD_USER_TO_CLIENT_BYTE: u8 = 72;
pub const REMOVE_USER_FROM_CLIENT_BYTE: u8 = 73;
pub const REQUEST_CALL_BYTE: u8 = 74;
pub const START_CALL_BYTE: u8 = 75;
pub const DENY_CALL_BYTE: u8 = 76;
pub const REQUEST_CALL_STREAM_ID_BYTE: u8 = 77;
pub const SEND_CALL_STREAM_ID_BYTE: u8 = 78;

pub async fn send_command_to_stream(
    cmd_byte: u8,
    subject: Option<String>,
    stream: &mut TcpStream,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    match subject {
        Some(subject) => {
            if subject.len() > 255 {
                return Err("Send Error: subject too long".into());
            }

            let mut message_bytes = vec![cmd_byte, subject.len() as u8];
            message_bytes.extend(subject.as_bytes());

            stream.write_all(&message_bytes).await?;
            stream.flush().await?;
        }
        None => {
            stream.write_all(&[cmd_byte]).await?;
            stream.flush().await?;
        }
    }

    return Ok(());
}

pub async fn receive_command_from_stream(
    stream: &mut TcpStream,
) -> Result<Option<(u8, Option<String>)>, Box<dyn Error + Send + Sync>> {
    let mut header = [0u8; 1];
    if stream.read_exact(&mut header).await.is_err() {
        return Ok(None);
    }
    let cmd = header[0];

    if matches!(
        cmd,
        ADD_USER_TO_CLIENT_BYTE
            | REMOVE_USER_FROM_CLIENT_BYTE
            | HELLO_FROM_CLIENT_BYTE
            | REQUEST_CALL_BYTE
            | START_CALL_BYTE
            | DENY_CALL_BYTE
            | REQUEST_CALL_STREAM_ID_BYTE
    ) {
        let mut len_buf = [0u8; 1];
        stream.read_exact(&mut len_buf).await?;
        let len = len_buf[0] as usize;

        let mut subject_buf = vec![0u8; len];
        stream.read_exact(&mut subject_buf).await?;

        let subject = from_utf8(&subject_buf)?.to_string();
        Ok(Some((cmd, Some(subject))))
    } else {
        Ok(Some((cmd, None)))
    }
}
