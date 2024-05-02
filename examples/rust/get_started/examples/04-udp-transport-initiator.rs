// This node routes a message, to a worker on a different node, over the udp transport.

use ockam::{node, route, Context, Result};
use ockam_transport_udp::{UdpBindArguments, UdpBindOptions, UdpTransportExtension, UDP};

#[ockam::node]
async fn main(ctx: Context) -> Result<()> {
    // Create a node with default implementations
    let mut node = node(ctx).await?;

    // Initialize the UDP Transport
    let udp = node.create_udp_transport().await?;

    let bind = udp.bind(UdpBindArguments::new(), UdpBindOptions::new()).await?;

    // Send a message to the "echoer" worker on a different node, over a udp transport.
    // Wait to receive a reply and print it.
    let r = route![bind, (UDP, "localhost:4000"), "echoer"];
    let reply = node.send_and_receive::<String>(r, "Hello Ockam!".to_string()).await?;

    println!("App Received: {}", reply); // should print "Hello Ockam!"

    // Stop all workers, stop the node, cleanup and return.
    node.stop().await
}
