// This node routes a message, to a worker on a different node, over the tcp transport.

use ockam::{route, Context, Result, TcpTransport, TCP};

#[ockam::node]
async fn main(mut ctx: Context) -> Result<()> {
    // Initialize the TCP Transport.
    let tcp = TcpTransport::create(&ctx).await?;

    // Create a TCP connection.
    tcp.connect("127.0.0.1:4000").await?;

    // Send a message to the "echoer" worker, on a different node, over a tcp transport.
    ctx.send(
        // route to the "echoer" worker, via a tcp connection.
        route![(TCP, "127.0.0.1:4000"), "echoer"],
        // the message you want echo-ed back
        "Hello Ockam!".to_string(),
    )
    .await?;

    // Wait to receive a reply and print it.
    let reply = ctx.receive::<String>().await?;
    println!("App Received: {}", reply); // should print "Hello Ockam!"

    // Stop all workers, stop the node, cleanup and return.
    ctx.stop().await
}
