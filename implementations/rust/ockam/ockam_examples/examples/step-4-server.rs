use ockam::{async_worker, Context, Result, Route, Routed, Worker};
use ockam_transport_tcp::{self as tcp, TcpRouter};
use std::net::SocketAddr;

struct EchoService;

const HUB_ADDRESS: &str = "127.0.0.1:4000";

#[async_worker]
impl Worker for EchoService {
    type Message = String;
    type Context = Context;

    async fn initialize(&mut self, ctx: &mut Self::Context) -> Result<()> {
        // Send a "register" event to the Hub. The hub will reply with a forwarding address.
        ctx.send_message(
            Route::new()
                .append_t(1, HUB_ADDRESS)
                .append("forwarding_service"),
            "register".to_string(),
        )
        .await
    }

    async fn handle_message(&mut self, ctx: &mut Context, msg: Routed<String>) -> Result<()> {
        if &msg.as_str() == &"register" {
            println!(
                "echo_service: My address on the hub is {}",
                msg.reply().recipient()
            );
            Ok(())
        } else {
            println!("echo_service: {}", msg);
            ctx.send_message(msg.reply(), msg.take()).await
        }
    }
}

#[ockam::node]
async fn main(ctx: Context) -> Result<()> {
    let router = TcpRouter::register(&ctx).await?;
    let hub_connection =
        tcp::start_tcp_worker(&ctx, HUB_ADDRESS.parse::<SocketAddr>().unwrap()).await?;

    router.register(&hub_connection).await?;

    ctx.start_worker("echo_service", EchoService).await
}
