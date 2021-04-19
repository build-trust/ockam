use ockam::{Context, RemoteForwarder, Result, SecureChannel};
use ockam_get_started::Echoer;
use ockam_transport_tcp::TcpTransport;
use ockam_vault::SoftwareVault;
use ockam_vault_sync_core::VaultWorker;

#[ockam::node]
async fn main(mut ctx: Context) -> Result<()> {
    let hub = "Paste the address of the node you created on Ockam Hub here.";

    let vault_address = VaultWorker::start(&ctx, SoftwareVault::default()).await?;

    SecureChannel::create_listener(&mut ctx, "secure_channel", vault_address).await?;

    let tcp = TcpTransport::create(&ctx).await?;
    tcp.connect(hub).await?;

    ctx.start_worker("echo_service", Echoer {}).await?;

    let mailbox = RemoteForwarder::create(&mut ctx, hub, "secure_channel").await?;
    println!(
        "Forwarding address for secure_channel: {}",
        mailbox.remote_address()
    );
    Ok(())
}
