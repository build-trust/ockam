use ockam::{async_worker, Context, RemoteForwarder, Result, Routed, SecureChannel, Worker};
use ockam_transport_tcp::TcpTransport;

const SECURE_CHANNEL: &str = "secure_channel";

struct EchoService;

#[async_worker]
impl Worker for EchoService {
    type Message = String;
    type Context = Context;

    async fn handle_message(&mut self, ctx: &mut Context, msg: Routed<String>) -> Result<()> {
        let return_route = msg.return_route();
        let msg_str = msg.body();
        println!("Server received message: {}", msg_str);

        ctx.send(return_route, msg_str.clone()).await?;
        println!("Server sent message: {}", msg_str);

        Ok(())
    }
}

#[ockam::node]
async fn main(mut ctx: ockam::Context) -> Result<()> {
    let hub = "127.0.0.1:4000";
    SecureChannel::create_listener(&mut ctx, SECURE_CHANNEL).await?;

    // Create and register a connection worker pair
    let tcp = TcpTransport::create(&ctx).await?;
    tcp.connect(hub).await?;

    ctx.start_worker("echo_server", EchoService {}).await?;

    let remote_forwarder = RemoteForwarder::create(&mut ctx, hub, SECURE_CHANNEL).await?;
    println!(
        "REMOTE_FORWARDER ADDRESS: {}",
        remote_forwarder.remote_address()
    );

    Ok(())
}
