use channel_examples::server_worker::Server;
use ockam::{RemoteForwarder, Result, SecureChannel, TcpTransport, Vault};

const SECURE_CHANNEL: &str = "xx_channel_listener";

#[ockam::node]
async fn main(mut ctx: ockam::Context) -> Result<()> {
    let vault_address = Vault::create(&ctx).await?;

    SecureChannel::create_listener(&mut ctx, SECURE_CHANNEL, &vault_address).await?;

    let hub_addr = "40.78.99.34:4000";

    // Create and register a connection worker pair
    let tcp = TcpTransport::create(&ctx).await?;
    tcp.connect(hub_addr).await?;

    let server = Server {};

    // Create the responder worker
    ctx.start_worker("echo_server", server).await?;

    let remote_forwarder = RemoteForwarder::create(&mut ctx, hub_addr, SECURE_CHANNEL).await?;
    println!(
        "PROXY REMOTE_FORWARDER: {}",
        remote_forwarder.remote_address()
    );

    // Crashes: ctx.stop().await

    Ok(())
}
