use ockam::{Context, RemoteForwarder, Result, Routed, TcpTransport, Worker};

struct EchoService;

#[ockam::worker]
impl Worker for EchoService {
    type Message = String;
    type Context = Context;

    async fn handle_message(&mut self, ctx: &mut Context, msg: Routed<String>) -> Result<()> {
        println!("echo_service: {}", msg);
        ctx.send(msg.return_route(), msg.body()).await
    }
}

#[ockam::node]
async fn main(ctx: Context) -> Result<()> {
    let hub = "Paste the address of the node you created on Ockam Hub here.";

    let tcp = TcpTransport::create(&ctx).await?;

    tcp.connect(hub).await?;

    ctx.start_worker("echo_service", EchoService).await?;

    let mailbox = RemoteForwarder::create(&ctx, hub, "echo_service").await?;

    println!(
        "echo_service forwarding address: {}",
        mailbox.remote_address()
    );

    Ok(())
}
