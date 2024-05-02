// This node creates an end-to-end encrypted secure channel over two udp transport hops.
// It then routes a message, to a worker on a different node, through this encrypted channel.

use ockam::identity::SecureChannelOptions;
use ockam::{node, route, Context, Result};
use ockam_transport_udp::{UdpBindArguments, UdpBindOptions, UdpTransportExtension};

#[ockam::node]
async fn main(ctx: Context) -> Result<()> {
    // Create a node with default implementations
    let mut node = node(ctx).await?;

    // Create an Identity to represent Alice.
    let alice = node.create_identity().await?;

    let udp = node.create_udp_transport().await?;
    let udp_bind = udp
        .bind(
            UdpBindArguments::new().with_peer_address("localhost:3000")?,
            UdpBindOptions::new(),
        )
        .await?;

    // Connect to a secure channel listener and perform a handshake.
    let r = route![udp_bind, "forward_to_bob", "bob_listener"];
    let channel = node
        .create_secure_channel(&alice, r, SecureChannelOptions::new())
        .await?;

    // Send a message to the echoer worker via the channel.
    // Wait to receive a reply and print it.
    let reply = node
        .send_and_receive::<String>(route![channel, "echoer"], "Hello Ockam!".to_string())
        .await?;
    println!("App Received: {}", reply); // should print "Hello Ockam!"

    // Stop all workers, stop the node, cleanup and return.
    node.stop().await
}
