/// This example uses the stream service to send messages between two
/// clients.  A stream is a buffered message sending channel, which
/// means that you can run `initiator` and `responder` in any order
/// you like.
use ockam::{stream::Stream, Context, Result, Route, SecureChannel, TcpTransport, Vault, TCP};
use std::time::Duration;

#[ockam::node]
async fn main(mut ctx: Context) -> Result<()> {
    let tcp = TcpTransport::create(&ctx).await?;
    tcp.connect("127.0.0.1:4000").await?;

    // Create a vault
    let vault = Vault::create(&ctx)?;

    // Create a bi-directional stream
    let (tx, _) = Stream::new(&ctx)?
        .with_interval(Duration::from_millis(100))
        .connect(
            Route::new().append_t(TCP, "127.0.0.1:4000"),
            // Stream name from THIS node to the OTHER node
            "test-a-b",
            // Stream name from OTHER to THIS
            "test-b-a",
        )
        .await?;

    // Create a secure channel via the stream
    let channel = SecureChannel::create(
        &ctx,
        Route::new()
            // Send via the stream
            .append(tx.clone())
            // And then to the secure_channel_listener
            .append("secure_channel_listener"),
        &vault,
    )
    .await?;

    // Send a message via the channel to the "printer"
    ctx.send(
        Route::new().append(channel.address()).append("echoer"),
        "Hello World!".to_string(),
    )
    .await?;

    // Wait for the reply
    let reply = ctx.receive_block::<String>().await?;
    println!("Reply via secure channel via stream: {}", reply);

    ctx.stop().await
}
