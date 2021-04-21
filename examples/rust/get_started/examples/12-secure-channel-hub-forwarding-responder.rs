use ockam::{Context, RemoteForwarder, Result, SecureChannel, Vault, TcpTransport};
use ockam_get_started::Echoer;

#[ockam::node]
async fn main(mut ctx: Context) -> Result<()> {
    let hub = "Paste the address of the node you created on Ockam Hub here.";

    let vault = Vault::create(&ctx).await?;

    SecureChannel::create_listener(&mut ctx, "secure_channel", &vault).await?;

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
