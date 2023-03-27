// This node routes a message, to a worker on a different node, over the tcp transport.

use ockam::{route, Context, Result, TcpConnectionTrustOptions, TcpTransport};

#[ockam::node]
async fn main(mut ctx: Context) -> Result<()> {
    // Initialize the TCP Transport.
    let tcp = TcpTransport::create(&ctx).await?;

    // Create a TCP connection to a different node.
    let connection_to_responder = tcp
        .connect("localhost:4000", TcpConnectionTrustOptions::insecure())
        .await?;

    // Send a message to the "echoer" worker on a different node, over a tcp transport.
    let r = route![connection_to_responder, "echoer"];
    ctx.send(r, "Hello Ockam!".to_string()).await?;

    // Wait to receive a reply and print it.
    let reply = ctx.receive::<String>().await?;
    println!("App Received: {}", reply); // should print "Hello Ockam!"

    // Stop all workers, stop the node, cleanup and return.
    ctx.stop().await
}
