// This node creates an end-to-end encrypted secure channel over two tcp transport hops.
// It then routes a message, to a worker on a different node, through this encrypted channel.

use ockam::identity::SecureChannelOptions;
use ockam::{node, route, Context, Result, TcpConnectionOptions};

#[ockam::node]
async fn main(ctx: Context) -> Result<()> {
    // Create an Identity to represent Alice.
    let mut node = node(ctx);
    let alice = node.create_identity().await?;

    // Create a TCP connection to the middle node.
    let tcp = node.create_tcp_transport().await?;
    let connection_to_middle_node = tcp.connect("localhost:3000", TcpConnectionOptions::new()).await?;

    // Connect to a secure channel listener and perform a handshake.
    let r = route![connection_to_middle_node, "forward_to_bob", "bob_listener"];
    let channel = node
        .create_secure_channel(&alice, r, SecureChannelOptions::new())
        .await?;

    // Send a message to the echoer worker via the channel.
    node.send(route![channel, "echoer"], "Hello Ockam!".to_string()).await?;

    // Wait to receive a reply and print it.
    let reply = node.receive::<String>().await?;
    println!("App Received: {}", reply); // should print "Hello Ockam!"

    // Stop all workers, stop the node, cleanup and return.
    node.stop().await
}
