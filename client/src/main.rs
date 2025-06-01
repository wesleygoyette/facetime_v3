use clap::{ArgAction, Parser};
use client::Client;
use shared::{TCP_PORT, UDP_PORT};
use std::{
    error::Error,
    io::{Write, stdout},
};
use tokio::io::{self, AsyncBufReadExt};

mod ascii_converter;
mod client;

#[derive(Parser, Debug)]
struct Args {
    #[arg(short, long)]
    username: Option<String>,

    #[arg(short, long, default_value = "facetime-v3.fly.dev")]
    server_address: String,

    #[arg(short, long, action = ArgAction::SetTrue)]
    auto_accept_calls: bool,

    #[arg(short, long, action = ArgAction::SetTrue)]
    border: bool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    let args = Args::parse();

    loop {
        let username = get_username(args.username.clone()).await?;

        let tcp_addr = format!("{}:{}", args.server_address, TCP_PORT);
        let udp_addr = format!("{}:{}", args.server_address, UDP_PORT);

        let mut client = Client::new(
            tcp_addr,
            udp_addr,
            username,
            args.auto_accept_calls,
            args.border,
        )
        .await?;
        match client.run().await? {
            Some(()) => continue,
            None => break,
        }
    }

    return Ok(());
}

async fn get_username(
    cli_username: Option<String>,
) -> Result<String, Box<dyn Error + Send + Sync>> {
    if let Some(name) = cli_username {
        return Ok(name);
    }

    print!("Enter username: ");
    stdout().flush()?;

    let mut lines = io::BufReader::new(io::stdin()).lines();
    match lines.next_line().await? {
        Some(name) if !name.trim().is_empty() => Ok(name),
        _ => Err("No username provided".into()),
    }
}
