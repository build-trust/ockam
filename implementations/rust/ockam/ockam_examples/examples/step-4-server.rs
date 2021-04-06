use ockam::{async_worker, Context, RemoteMailbox, Result, Routed, Worker};
use ockam_transport_tcp::{self as tcp, TcpRouter};
use std::net::SocketAddr;

struct EchoService;

const HUB_ADDRESS: &str = "127.0.0.1:4000";

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
async fn main(mut ctx: Context) -> Result<()> {
    let router = TcpRouter::register(&ctx).await?;
    let hub_connection =
        tcp::start_tcp_worker(&ctx, HUB_ADDRESS.parse::<SocketAddr>().unwrap()).await?;

    router.register(&hub_connection).await?;

    ctx.start_worker("echo_service", EchoService).await?;

    let remote_mailbox_info = RemoteMailbox::<String>::create(
        &mut ctx,
        HUB_ADDRESS.parse::<SocketAddr>().unwrap(),
        "echo_service",
    )
    .await?;
    println!(
        "echo_service: My address on the hub is {}",
        remote_mailbox_info.alias_address()
    );

    Ok(())
}
