use std::{error::Error, io::{stdout, Write}};
use clap::Parser;
use client::Client;
use shared::TCP_PORT;
use tokio::io::{self, AsyncBufReadExt};

// use ascii_converter::AsciiConverter;
// use crossterm::{cursor::MoveTo, execute, terminal::ClearType, terminal::Clear};
// use opencv::{core::{Mat, MatTraitConst}, videoio::{VideoCapture, VideoCaptureTrait, VideoCaptureTraitConst, CAP_ANY}};

mod ascii_converter;
mod client;

#[derive(Parser, Debug)]
struct Args {
    #[arg(short, long)]
    username: Option<String>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    let args = Args::parse();
    let username = get_username(args.username).await?;

    let addr = format!("127.0.0.1:{}", TCP_PORT);
    let mut client = Client::new(addr, username).await?;
    client.run().await?;

    return Ok(());

    // let mut cam = VideoCapture::new(0, CAP_ANY)?;

    // if !cam.is_opened()? {
    //     eprintln!("Error: Could not open camera");
    //     return Ok(());
    // }

    // let ascii_converter = AsciiConverter::new(120, 40);

    // println!("Starting camera ASCII feed... Press Ctrl+C to exit");
    // println!("Camera initialized successfully!");

    // let mut frame = Mat::default();

    // loop {

    //     cam.read(&mut frame)?;

    //     if frame.empty() {
    //         eprintln!("Warning: Empty frame captured");
    //         continue;
    //     }

    //     let message = ascii_converter.frame_to_ascii(&frame)?;

    //     execute!(stdout(), Clear(ClearType::All), MoveTo(0, 0))?;
    //     stdout().flush()?;
    //     print!("{}", message);
    //     stdout().flush()?;
    // }
}

async fn get_username(cli_username: Option<String>) -> Result<String, Box<dyn Error + Send + Sync>> {
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
