use channel_examples::server_worker::Server;
use ockam::{RemoteMailbox, Result, SecureChannel, SecureChannelListenerMessage};
use ockam_transport_tcp::TcpTransport;
use std::net::SocketAddr;
use std::str::FromStr;

const XX_CHANNEL_LISTENER_ADDRESS: &str = "xx_channel_listener";

#[ockam::node]
async fn main(mut ctx: ockam::Context) -> Result<()> {
    SecureChannel::create_listener(&mut ctx, XX_CHANNEL_LISTENER_ADDRESS).await?;

    // let hub_addr = SocketAddr::from_str("138.91.152.195:4000").unwrap();
    let hub_addr = SocketAddr::from_str("127.0.0.1:4000").unwrap();

    // Create and register a connection worker pair
    TcpTransport::create(&ctx, hub_addr).await?;

    let server = Server {};

    // Create the responder worker
    ctx.start_worker("echo_server", server).await?;

    let mailbox = RemoteMailbox::<SecureChannelListenerMessage>::create(
        &mut ctx,
        hub_addr,
        XX_CHANNEL_LISTENER_ADDRESS,
    )
    .await?;
    println!("PROXY ADDRESS: {}", mailbox.remote_address());

    // Crashes: ctx.stop().await

    Ok(())
}
