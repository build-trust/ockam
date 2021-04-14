use ockam::{
    async_worker, Context, RemoteMailbox, Result, Routed, SecureChannel,
    SecureChannelListenerMessage, Worker,
};
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
    let hub = "104.42.24.183:4000";
    SecureChannel::create_listener(&mut ctx, SECURE_CHANNEL).await?;

    // Create and register a connection worker pair
    let tcp = TcpTransport::create(&ctx).await?;
    tcp.connect(hub).await?;

    ctx.start_worker("echo_server", EchoService {}).await?;

    let mailbox =
        RemoteMailbox::<SecureChannelListenerMessage>::create(&mut ctx, hub, SECURE_CHANNEL)
            .await?;
    println!("PROXY ADDRESS: {}", mailbox.remote_address());

    Ok(())
}
