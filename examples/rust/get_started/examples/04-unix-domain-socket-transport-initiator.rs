// This node routes a message, to a worker on a different node, over the tcp transport.

use ockam::{node, route, Context, Result};
use ockam_transport_uds::{UdsTransportExtension, UDS};

#[ockam::node]
async fn main(ctx: Context) -> Result<()> {
    // Create a node with default implementations
    let mut node = node(ctx).await?;

    // Initialize the UDS Transport
    let uds = node.create_uds_transport().await?;

    let connection = uds.connect("/tmp/ockam-example-echoer").await;

    if let Err(e) = connection {
        println!("Error connecting to echoer {e}");
    }
    // Send a message to the "echoer" worker, on a different node, over a uds transport.
    let r = route![(UDS, "/tmp/ockam-example-echoer"), "echoer"];
    node.send(r, "Hello Ockam!".to_string()).await?;

    // Wait to receive a reply and print it.
    let reply = node.receive::<String>().await?;
    println!("App Received: {}", reply.into_body()?); // should print "Hello Ockam!"

    // Stop all workers, stop the node, cleanup and return.
    node.shutdown().await
}
