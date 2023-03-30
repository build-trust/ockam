// This node routes a message, to a worker on a different node, over two tcp transport hops.

use ockam::{route, Context, Result, TcpConnectionTrustOptions, TcpTransport};

#[ockam::node]
async fn main(mut ctx: Context) -> Result<()> {
    // Initialize the TCP Transport.
    let tcp = TcpTransport::create(&ctx).await?;

    // Create a TCP connection to the middle node.
    let connection_to_middle_node = tcp
        .connect("localhost:3000", TcpConnectionTrustOptions::insecure_test())
        .await?;

    // Send a message to the "echoer" worker, on a different node, over two tcp hops.
    let r = route![connection_to_middle_node, "forward_to_responder", "echoer"];
    ctx.send(r, "Hello Ockam!".to_string()).await?;

    // Wait to receive a reply and print it.
    let reply = ctx.receive::<String>().await?;
    println!("App Received: {}", reply); // should print "Hello Ockam!"

    // Stop all workers, stop the node, cleanup and return.
    ctx.stop().await
}
