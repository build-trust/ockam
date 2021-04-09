use channel_examples::client_worker::Client;
use ockam::Result;
use ockam_transport_tcp::TcpTransport;
use std::net::SocketAddr;
use std::str::FromStr;

#[ockam::node]
async fn main(ctx: ockam::Context) -> Result<()> {
    // let hub_addr = SocketAddr::from_str("138.91.152.195:4000").unwrap();
    let hub_addr = SocketAddr::from_str("127.0.0.1:4000").unwrap();

    // Create and register a connection worker pair
    TcpTransport::create(&ctx, hub_addr).await?;

    let client = Client::new(hub_addr, "27164a70".to_string());

    ctx.start_worker("echo_client", client).await?;

    // Crashes: ctx.stop().await

    Ok(())
}
