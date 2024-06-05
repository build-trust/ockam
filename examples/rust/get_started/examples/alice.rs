use ockam::identity::SecureChannelOptions;
use ockam::tcp::{TcpConnectionOptions, TcpTransportExtension};
use ockam::{node, route, Context, Result};
use std::io;

#[ockam::node]
async fn main(ctx: Context) -> Result<()> {
    // Create a node with default implementations
    let mut node = node(ctx).await?;
    // Initialize the TCP Transport
    let tcp = node.create_tcp_transport().await?;

    // Create an Identity to represent Alice.
    let alice = node.create_identity().await?;

    // This program expects that Bob has setup a forwarding address,
    // for his secure channel listener, on the Ockam node at 1.node.ockam.network:4000.
    //
    // From standard input, read this forwarding address for Bob's secure channel listener.
    println!("\nEnter the forwarding address for Bob: ");
    let mut address = String::new();
    io::stdin().read_line(&mut address).expect("Error reading from stdin.");
    let forwarding_address = address.trim();

    // Combine the tcp address of the node and the forwarding_address to get a route
    // to Bob's secure channel listener.
    let node_in_hub = tcp
        .connect("1.node.ockam.network:4000", TcpConnectionOptions::new())
        .await?;
    let route_to_bob_listener = route![node_in_hub, forwarding_address, "listener"];

    // As Alice, connect to Bob's secure channel listener, and perform an
    // Authenticated Key Exchange to establish an encrypted secure channel with Bob.
    let channel = node
        .create_secure_channel(&alice, route_to_bob_listener, SecureChannelOptions::new())
        .await?;

    println!("\n[✓] End-to-end encrypted secure channel was established.\n");

    loop {
        // Read a message from standard input.
        println!("Type a message for Bob's echoer:");
        let mut message = String::new();
        io::stdin().read_line(&mut message).expect("Error reading from stdin.");
        let message = message.trim();

        // Send the provided message, through the channel, to Bob's echoer.
        node.send(route![channel.clone(), "echoer"], message.to_string())
            .await?;

        // Wait to receive an echo and print it.
        let reply = node.receive::<String>().await?;
        println!("Alice received an echo: {}\n", reply.into_body()?); // should print "Hello Ockam!"
    }

    // This program will keep running until you stop it with Ctrl-C
}
