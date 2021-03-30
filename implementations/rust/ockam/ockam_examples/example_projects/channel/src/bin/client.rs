use channel_examples::client_worker::Client;
use ockam::Result;
use ockam_transport_tcp::{start_tcp_worker, TcpRouter};
use std::net::SocketAddr;
use std::str::FromStr;

#[ockam::node]
async fn main(ctx: ockam::Context) -> Result<()> {
    // Create and register a TcpRouter
    let rh = TcpRouter::register(&ctx).await?;

    // let hub_addr = SocketAddr::from_str("138.91.152.195:4000").unwrap();
    let hub_addr = SocketAddr::from_str("127.0.0.1:4000").unwrap();
    // Create and register a connection worker pair
    let w_pair = start_tcp_worker(&ctx, hub_addr).await?;
    rh.register(&w_pair).await?;

    let channel_id = "random_id".to_string();
    let client = Client::new(channel_id, hub_addr, "91e7e94c".to_string());

    ctx.start_worker("echo_client", client).await?;

    // Crashes: ctx.stop().await

    Ok(())
}
