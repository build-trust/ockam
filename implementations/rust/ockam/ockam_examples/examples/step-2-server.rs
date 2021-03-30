use ockam::{async_worker, Context, Result, Routed, Worker};
use ockam_transport_tcp::TcpRouter;
use std::net::SocketAddr;

struct EchoService;

#[async_worker]
impl Worker for EchoService {
    type Message = String;
    type Context = Context;

    async fn handle_message(&mut self, ctx: &mut Context, msg: Routed<String>) -> Result<()> {
        println!("echo_service: {}", msg);
        ctx.send_message(msg.reply(), msg.take()).await
    }
}

#[ockam::node]
async fn main(ctx: Context) -> Result<()> {
    let _router = TcpRouter::bind(&ctx, "127.0.0.1:10222".parse::<SocketAddr>().unwrap()).await?;
    ctx.start_worker("echo_service", EchoService).await
}
