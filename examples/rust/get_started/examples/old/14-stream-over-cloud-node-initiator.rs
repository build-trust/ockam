/// This example uses the stream service to send messages between two
/// clients.  A stream is a buffered message sending channel, which
/// means that you can run `initiator` and `responder` in any order
/// you like.
use ockam::{route, stream::Stream, Context, Result, TcpTransport, TCP};

#[ockam::node]
async fn main(ctx: Context) -> Result<()> {
    let mut node = node(ctx);
    let _tcp = node.create_tcp_transport().await?;

    let (sender, _receiver) = node.create_stream().await?
        .connect(
            route![(TCP, "localhost:4000")],
            // Stream name from THIS node to the OTHER node
            "test-a-b",
            // Stream name from OTHER to THIS
            "test-b-a",
        )
        .await?;

    node.send(
        sender.to_route().append("echoer"),
        "Hello World!".to_string(),
    )
        .await?;

    let reply = node.receive::<String>().await?;
    println!("Reply via stream: {}", reply);

    node.stop().await
}
