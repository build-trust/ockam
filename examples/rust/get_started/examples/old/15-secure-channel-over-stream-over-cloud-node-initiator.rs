/// This example uses the stream service to send messages between two
/// clients.  A stream is a buffered message sending channel, which
/// means that you can run `initiator` and `responder` in any order
/// you like.
use ockam::{route, stream::Stream, Context, Result, SecureChannel, TcpTransport, Vault, TCP};

#[ockam::node]
async fn main(mut ctx: Context) -> Result<()> {
    let _tcp = TcpTransport::create(&ctx).await?;

    // Create a vault
    let vault = Vault::create(&ctx).await?;

    // Create a bi-directional stream
    let (sender, _receiver) = Stream::new(&ctx).await?
        .client_id("secure-channel-over-stream-over-cloud-node-initiator")
        .connect(
            route![(TCP, "localhost:4000")],
            // Stream name from THIS node to the OTHER node
            "test-a-b",
            // Stream name from OTHER to THIS
            "test-b-a",
        )
        .await?;

    // Create a secure channel via the stream
    let channel = SecureChannel::create(
        &ctx,
        route![
            // Send via the stream
            sender.clone(),
            // And then to the secure_channel_listener
            "secure_channel_listener"
        ],
        &vault,
    )
    .await?;

    // Send a message via the channel to the echoer worker
    ctx.send(
        route![channel.address(), "echoer"],
        "Hello World!".to_string(),
    )
    .await?;

    // Wait for the reply
    let reply = ctx.receive_block::<String>().await?;
    println!("Reply via secure channel via stream: {}", reply);

    ctx.stop().await
}
