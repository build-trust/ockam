// This node routes a message, to a worker on a cloud node, over the tcp transport.

use ockam::{route, Context, Result, TcpTransport, TCP};

#[ockam::node]
async fn main(ctx: Context) -> Result<()> {
    // Create a cloud node by going to https://hub.ockam.network
    let cloud_node_tcp_address = "Paste the tcp address of your cloud node here.";

    // Initialize the TCP Transport.
    let node = node(ctx);
    let tcp = node.create_tcp_transport().await?;

    // Send a message to the `echo` worker on your cloud node.
    node.send(
        // route to the echo worker on your cloud node
        route![(TCP, cloud_node_tcp_address), "echo"],
        // the message you want echo-ed back
        "Hello Ockam!".to_string(),
    )
        .await?;

    // Wait to receive the echo and print it.
    let msg = node.receive::<String>().await?;
    println!("App Received: '{}'", msg); // should print "Hello Ockam!"

    // Stop the node.
    node.stop().await
}
