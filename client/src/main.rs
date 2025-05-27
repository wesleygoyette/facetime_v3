use std::error::Error;

use clap::Parser;
use client::Client;
use shared::TCP_PORT;

mod ascii_converter;
mod client;

#[derive(Parser, Debug)]
struct Args {
    #[arg(short, long, default_value = "Wesley")]
    username: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    let username = Args::parse().username;

    let addr = format!("127.0.0.1:{}", TCP_PORT);

    let mut client = Client::new(addr, username).await?;

    client.run().await?;

    return Ok(());
}
