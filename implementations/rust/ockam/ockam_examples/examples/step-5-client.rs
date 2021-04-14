use ockam::{Context, Result, Route, SecureChannel};
use ockam_transport_tcp::{TcpTransport, TCP};

#[ockam::node]
async fn main(mut ctx: Context) -> Result<()> {
    let remote_node = "104.42.24.183:4000";
    let echo_service = "0f186d34";

    TcpTransport::create(&ctx, remote_node).await?;
    let channel_info = SecureChannel::create(
        &mut ctx,
        Route::new()
            .append_t(TCP, remote_node)
            .append(echo_service)
            .append("secure_channel"),
    )
    .await?;

    ctx.send(
        Route::new()
            .append(channel_info.address().clone())
            .append("echo_service"),
        "Hello Ockam!".to_string(),
    )
    .await?;

    // Then wait for a message back!
    let msg = ctx.receive::<String>().await?;
    println!("Received echo: '{}'", msg);
    ctx.stop().await
}
