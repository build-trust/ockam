// This node routes a message, to a different node, using a forwarding address on the cloud node.

use ockam::{route, Context, Result, TcpTransport, TCP};

#[ockam::node]
async fn main(ctx: Context) -> Result<()> {
    // Create a cloud node by going to https://hub.ockam.network
    let cloud_node_tcp_address = "Paste the tcp address of your cloud node here.";

    // Run 11-forwarding-via-a-cloud-node-responder,
    // it will print the forwarding address of echoer on your cloud node
    let echoer_forwarding_address = "Paste the forwarding address of the echoer here.";

    // Initialize the TCP Transport.
    let node = node(ctx);
    let _tcp = node.create_tcp_transport().await?;

    // Send a message to the echoer worker, on a different node,
    // using a forwarding address on your cloud node
    node.send(
        route![(TCP, cloud_node_tcp_address), echoer_forwarding_address],
        "Hello Ockam!".to_string(),
    )
        .await?;

    // Wait to receive a reply and print it.
    let reply = node.receive::<String>().await?;
    println!("App Received: {}", reply); // should print "Hello Ockam!"

    // Stop all workers, stop the node, cleanup and return.
    node.stop().await
}
