use ockam::{Context, Result, Route, SecureChannel, SecureChannelMessage};
use ockam_transport_tcp::{self as tcp, TcpRouter};
use std::net::SocketAddr;

const XX_CHANNEL_LISTENER_ADDRESS: &str = "xx_channel_listener";

#[ockam::node]
async fn main(mut ctx: Context) -> Result<()> {
    let remote_node = "Paste the address of the node you created on Ockam Hub here.";
    let echo_service =
        "Paste the forwarded address that the server received from registration here.";

    // Create and register a connection
    let router = TcpRouter::register(&ctx).await?;
    let connection =
        tcp::start_tcp_worker(&ctx, remote_node.parse::<SocketAddr>().unwrap()).await?;
    router.register(&connection).await?;

    let channel_info = SecureChannel::create(
        &mut ctx,
        Route::new()
            .append_t(1, remote_node)
            .append(echo_service)
            .append(XX_CHANNEL_LISTENER_ADDRESS),
    )
    .await?;

    ctx.send_message(
        Route::new()
            .append(channel_info.worker_address().clone())
            .append("echo_service"),
        SecureChannelMessage::create("Hello Ockam!".to_string()).unwrap(),
    )
    .await?;

    // Then wait for a message back!
    let msg = ctx.receive::<String>().await?;
    println!("Received echo: '{}'", msg);
    ctx.stop().await
}
