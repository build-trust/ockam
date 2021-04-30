use ockam::{
    async_worker, Context, RemoteForwarder, Result, Routed, SecureChannel, TcpTransport, Vault,
    VaultSync, Worker, XXNewKeyExchanger,
};

struct EchoService;

#[async_worker]
impl Worker for EchoService {
    type Message = String;
    type Context = Context;

    async fn handle_message(&mut self, ctx: &mut Context, msg: Routed<String>) -> Result<()> {
        ctx.send(msg.return_route(), msg.body()).await
    }
}

#[ockam::node]
async fn main(mut ctx: Context) -> Result<()> {
    let hub = "40.78.99.34:4000";

    let tcp = TcpTransport::create(&ctx).await?;

    tcp.connect(hub).await?;

    let vault_address = Vault::create(&ctx)?;

    let vault_sync = VaultSync::create_with_worker(&ctx, &vault_address, "FIXME").unwrap();
    let xx_key_exchanger = XXNewKeyExchanger::new(vault_sync.clone());
    SecureChannel::create_listener(&ctx, "secure_channel", xx_key_exchanger, vault_sync).await?;

    ctx.start_worker("echo_service", EchoService).await?;

    let mailbox = RemoteForwarder::create(&mut ctx, hub, "secure_channel").await?;

    println!(
        "echo_service secure channel address: {}",
        mailbox.remote_address()
    );
    Ok(())
}
