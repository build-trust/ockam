// This node routes a message, to a worker on a different node, over the udp transport.

use ockam::{node, route, Context, Result};
use ockam_transport_udp::{UdpTransportExtension, UDP};

#[ockam::node]
async fn main(ctx: Context) -> Result<()> {
    // Create a node with default implementations
    let mut node = node(ctx);

    // Initialize the UDP Transport
    let _udp = node.create_udp_transport().await?;

    // Send a message to the "echoer" worker, on a different node, over an udp transport.
    let r = route![(UDP, "localhost:4000"), "echoer"];
    node.send(r, "Hello Ockam!".to_string()).await?;

    // Wait to receive a reply and print it.
    let reply = node.receive::<String>().await?;
    println!("App Received: {}", reply); // sohuld print "Hello Ockam!"

    // Stop all workers, stop the node, cleanup and return.
    node.stop().await
}
