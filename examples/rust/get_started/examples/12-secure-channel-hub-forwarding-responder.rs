use ockam::{Context, RemoteMailbox, Result, SecureChannel, SecureChannelListenerMessage};
use ockam_get_started::Echoer;
use ockam_transport_tcp::TcpTransport;

#[ockam::node]
async fn main(mut ctx: Context) -> Result<()> {
    let hub = "Paste the address of the node you created on Ockam Hub here.";

    SecureChannel::create_listener(&mut ctx, "secure_channel").await?;

    let tcp = TcpTransport::create(&ctx).await?;
    tcp.connect(hub).await?;

    ctx.start_worker("echo_service", Echoer {}).await?;

    let mailbox =
        RemoteMailbox::<SecureChannelListenerMessage>::create(&mut ctx, hub, "secure_channel")
            .await?;
    println!(
        "Forwarding address for secure_channel: {}",
        mailbox.remote_address()
    );
    Ok(())
}
