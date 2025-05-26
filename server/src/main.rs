use shared::TCP_PORT;
use std::error::Error;
use wes_sfu::WeSFU;

mod wes_sfu;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let addr = format!("0.0.0.0:{}", TCP_PORT);

    let server = WeSFU::new(addr).await?;

    server.run().await?;

    return Ok(());
}
