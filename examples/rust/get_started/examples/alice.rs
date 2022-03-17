use ockam::{route, Context, Identity, Result, TrustEveryonePolicy, Vault};
use ockam::{TcpTransport, TCP};
use std::io;

#[ockam::node]
async fn main(mut ctx: Context) -> Result<()> {
    // Initialize the TCP Transport.
    TcpTransport::create(&ctx).await?;

    // Create a Vault to safely store secret keys for Alice.
    let vault = Vault::create();

    // Create an Identity to represent Alice.
    let alice = Identity::create(&ctx, &vault).await?;

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
    let route_to_bob_listener = route![(TCP, "1.node.ockam.network:4000"), forwarding_address, "listener"];

    // As Alice, connect to Bob's secure channel listener, and perform an
    // Authenticated Key Exchange to establish an encrypted secure channel with Bob.
    let channel = alice
        .create_secure_channel(route_to_bob_listener, TrustEveryonePolicy)
        .await?;

    println!("\n[✓] End-to-end encrypted secure channel was established.\n");

    loop {
        // Read a message from standard input.
        println!("Type a message for Bob's echoer:");
        let mut message = String::new();
        io::stdin().read_line(&mut message).expect("Error reading from stdin.");
        let message = message.trim();

        // Send the provided message, through the channel, to Bob's echoer.
        ctx.send(route![channel.clone(), "echoer"], message.to_string()).await?;

        // Wait to receive an echo and print it.
        let reply = ctx.receive::<String>().await?;
        println!("Alice received an echo: {}\n", reply); // should print "Hello Ockam!"
    }

    // This program will keep running until you stop it with Ctrl-C
}
