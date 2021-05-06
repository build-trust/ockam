use ockam::{Context, RemoteForwarder, Result, Routed, SecureChannel, TcpTransport, Vault, Worker};

struct EchoService;

#[ockam::worker]
impl Worker for EchoService {
    type Message = String;
    type Context = Context;

    async fn handle_message(&mut self, ctx: &mut Context, msg: Routed<String>) -> Result<()> {
        ctx.send(msg.return_route(), msg.body()).await
    }
}

#[ockam::node]
async fn main(mut ctx: Context) -> Result<()> {
    let hub = "Paste the address of the node you created on Ockam Hub here.";

    let tcp = TcpTransport::create(&ctx).await?;

    tcp.connect(hub).await?;

    let vault_address = Vault::create(&ctx)?;

    SecureChannel::create_listener(&ctx, "secure_channel", &vault_address).await?;

    ctx.start_worker("echo_service", EchoService).await?;

    let mailbox = RemoteForwarder::create(&mut ctx, hub, "secure_channel").await?;

    println!(
        "echo_service secure channel address: {}",
        mailbox.remote_address()
    );
    Ok(())
}
