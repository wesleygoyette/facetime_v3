use shared::{TCP_PORT, UDP_PORT};
use std::error::Error;
use wes_sfu::WeSFU;

mod wes_sfu;

const TCP_BIND_ADDR: &str = "0.0.0.0";
const UDP_BIND_ADDR: &str = "fly-global-services";

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let tcp_addr = format!("{}:{}", TCP_BIND_ADDR, TCP_PORT);
    let udp_addr = format!("{}:{}", UDP_BIND_ADDR, UDP_PORT);

    let server = WeSFU::new(tcp_addr, udp_addr).await?;

    server.run().await?;

    return Ok(());
}
