use channel_examples::server_worker::Server;
use ockam::{RemoteForwarder, Result, SecureChannel};
use ockam_transport_tcp::TcpTransport;
use ockam_vault::SoftwareVault;
use ockam_vault_sync_core::VaultWorker;

const SECURE_CHANNEL: &str = "xx_channel_listener";

#[ockam::node]
async fn main(mut ctx: ockam::Context) -> Result<()> {
    let vault_address = VaultWorker::start(&ctx, SoftwareVault::default()).await?;

    SecureChannel::create_listener(&mut ctx, SECURE_CHANNEL, vault_address).await?;

    let hub_addr = "104.42.24.183:4000";

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
