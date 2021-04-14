use ockam::{async_worker, Context, Result, Routed, Worker};
use ockam_transport_tcp::TcpTransport;

struct EchoService;

#[async_worker]
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
    TcpTransport::create_listener(&ctx, "127.0.0.1:10222").await?;
    ctx.start_worker("echo_service", EchoService).await
}
