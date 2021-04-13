use ockam::{Context, Result, Route};
use ockam_transport_tcp::{self as tcp, TcpRouter, TCP};
use std::net::SocketAddr;

#[ockam::node]
async fn main(mut ctx: Context) -> Result<()> {
    let remote_node = "Paste the address of the node you created on Ockam Hub here.";

    // Create and register a connection
    let router = TcpRouter::register(&ctx).await?;
    let connection =
        tcp::start_tcp_worker(&ctx, remote_node.parse::<SocketAddr>().unwrap()).await?;
    router.register(&connection).await?;

    ctx.send_message(
        Route::new()
            .append_t(TCP, remote_node)
            .append("echo_service"),
        "Hello Ockam!".to_string(),
    )
    .await?;

    // Then wait for a message back!
    let msg = ctx.receive::<String>().await?;
    println!("Received return message: '{}'", msg);
    ctx.stop().await
}
