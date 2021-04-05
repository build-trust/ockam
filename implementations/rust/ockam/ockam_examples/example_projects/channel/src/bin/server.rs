use channel_examples::server_worker::Server;
use ockam::Result;
use ockam_channel::{ChannelListenerMessage, HubProxy, ProxyRegistered, XXChannelListener};
use ockam_transport_tcp::{start_tcp_worker, TcpRouter};
use std::net::SocketAddr;
use std::str::FromStr;

const XX_CHANNEL_LISTENER_ADDRESS: &str = "xx_channel_listener";

#[ockam::node]
async fn main(mut ctx: ockam::Context) -> Result<()> {
    let xx_channel_listener = XXChannelListener::new();
    ctx.start_worker(XX_CHANNEL_LISTENER_ADDRESS, xx_channel_listener)
        .await
        .unwrap();

    // Create and register a TcpRouter
    let rh = TcpRouter::register(&ctx).await?;

    // let hub_addr = SocketAddr::from_str("138.91.152.195:4000").unwrap();
    let hub_addr = SocketAddr::from_str("127.0.0.1:4000").unwrap();
    // Create and register a connection worker pair
    let w_pair = start_tcp_worker(&ctx, hub_addr).await?;
    rh.register(&w_pair).await?;

    let server = Server {};

    // Create the responder worker
    ctx.start_worker("echo_server", server).await?;

    HubProxy::<ChannelListenerMessage>::register_proxy(
        &ctx,
        "proxy".into(),
        hub_addr,
        XX_CHANNEL_LISTENER_ADDRESS.into(),
    )
    .await?;
    let proxy_address = ctx.receive::<ProxyRegistered>().await?.take().take();
    println!("PROXY ADDRESS: {}", proxy_address.address());

    // Crashes: ctx.stop().await

    Ok(())
}
