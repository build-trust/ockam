// This node creates a udp connection to a node at 127.0.0.1:4000
// Starts a relay worker to forward messages to 127.0.0.1:4000
// Starts a udp bind at 127.0.0.1:3000
// It then runs forever waiting to route messages.

use hello_ockam::Relay;
use ockam::{node, Context, Result};
use ockam_core::route;
use ockam_transport_udp::{UdpBindArguments, UdpBindOptions, UdpTransportExtension, UDP};

#[ockam::node]
async fn main(ctx: Context) -> Result<()> {
    // Create a node with default implementations
    let node = node(ctx).await?;

    // Initialize the UDP Transport
    let udp = node.create_udp_transport().await?;
    let udp_bind = udp
        .bind(
            UdpBindArguments::new().with_bind_address("127.0.0.1:3000")?,
            UdpBindOptions::new(),
        )
        .await?;

    // Start a Relay to forward messages to Bob using the TCP connection.
    node.start_worker(
        "forward_to_bob",
        Relay::new(route![udp_bind.clone(), (UDP, "127.0.0.1:4000")]),
    )?;

    node.flow_controls()
        .add_consumer(&"forward_to_bob".into(), udp_bind.flow_control_id());

    // Don't call node.shutdown() here so this node runs forever.
    Ok(())
}
