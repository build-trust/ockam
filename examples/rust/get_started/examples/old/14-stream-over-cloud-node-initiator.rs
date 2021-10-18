/// This example uses the stream service to send messages between two
/// clients.  A stream is a buffered message sending channel, which
/// means that you can run `initiator` and `responder` in any order
/// you like.
use ockam::{route, stream::Stream, Context, Result, TcpTransport, TCP};

#[ockam::node]
async fn main(mut ctx: Context) -> Result<()> {
    let _tcp = TcpTransport::create(&ctx).await?;

    let (sender, _receiver) = Stream::new(&ctx).await?
        .connect(
            route![(TCP, "localhost:4000")],
            // Stream name from THIS node to the OTHER node
            "test-a-b",
            // Stream name from OTHER to THIS
            "test-b-a",
        )
        .await?;

    ctx.send(
        sender.to_route().append("echoer"),
        "Hello World!".to_string(),
    )
    .await?;

    let reply = ctx.receive_block::<String>().await?;
    println!("Reply via stream: {}", reply);

    ctx.stop().await
}
