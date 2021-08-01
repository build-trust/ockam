# End-to-End Encryption with Rust

```rust
// examples/bob.rs

use ockam::{Context, Entity, Result, SecureChannels, TrustEveryonePolicy, Vault};
use ockam::{RemoteForwarder, Routed, TcpTransport, Worker, TCP};

struct Echoer;

#[ockam::worker]
impl Worker for Echoer {
    type Context = Context;
    type Message = String;

    async fn handle_message(&mut self, ctx: &mut Context, msg: Routed<String>) -> Result<()> {
        println!("\n[✓] Address: {}, Received: {}", ctx.address(), msg);

        // Echo the message body back on its return_route.
        ctx.send(msg.return_route(), msg.body()).await
    }
}

#[ockam::node]
async fn main(ctx: Context) -> Result<()> {
    // Initialize the TCP Transport.
    TcpTransport::create(&ctx).await?;

    // Create a Vault to safely store secret keys for Bob.
    let vault = Vault::create(&ctx).expect("failed to create vault");

    // Create an Entity to represent Bob.
    let mut bob = Entity::create(&ctx, &vault)?;

    // Create a secure channel listener at address "listener"
    bob.create_secure_channel_listener("listener", TrustEveryonePolicy)?;

    // The computer that is running this program is likely within a private network and
    // not accessible over the internet.
    //
    // To allow Alice and others to initiate an end-to-end secure channel with this program
    // we connect with 1.node.ockam.network:4000 as a TCP client and ask the forwarding
    // service on that node to create a forwarder for us.
    //
    // All messages that arrive at that forwarding address will be send to this program
    // using the TCP connection we created as a client.
    let node_in_hub = (TCP, "1.node.ockam.network:4000");
    let forwarder = RemoteForwarder::create(&ctx, node_in_hub, "listener").await?;
    println!("\n[✓] RemoteForwarder was created on the node at: 1.node.ockam.network:4000");
    println!("Forwarding address of Bob's secure channel listener is:");
    println!("{}", forwarder.remote_address());

    // Start a worker, of type Echoer, at address "echoer".
    // This worker will echo back every message it receives, along its return route.
    ctx.start_worker("echoer", Echoer).await?;

    // We won't call ctx.stop() here, this program will run until you stop it with Ctrl-C
    Ok(())
}

```

```rust
// examples/alice.rs

use ockam::{route, Context, Entity, Result, SecureChannels, TrustEveryonePolicy, Vault};
use ockam::{TcpTransport, TCP};
use std::io;

#[ockam::node]
async fn main(mut ctx: Context) -> Result<()> {
    // Initialize the TCP Transport.
    TcpTransport::create(&ctx).await?;

    // Create a Vault to safely store secret keys for Alice.
    let vault = Vault::create(&ctx).expect("failed to create vault");

    // Create an Entity to represent Alice.
    let mut alice = Entity::create(&ctx, &vault)?;

    // This program expects that Bob has setup a forwarding address,
    // for his secure channel listener, on the Ockam node at 1.node.ockam.network:4000.
    //
    // From standard input, read this forwarding address for Bob's secure channel listener.
    println!("\nEnter the forwarding address of Bob's secure channel listener: ");
    let mut address = String::new();
    io::stdin().read_line(&mut address).expect("Error reading from stdin.");
    let forwarding_address = address.trim();

    // Combine the tcp address of the node and the forwarding_address to get a route
    // to Bob's secure channel listener.
    let route_to_bob_listener = route![(TCP, "1.node.ockam.network:4000"), forwarding_address];

    // As Alice, connect to Bob's secure channel listener, and perform an
    // Authenticated Key Exchange to establish an encrypted secure channel with Bob.
    let channel = alice.create_secure_channel(route_to_bob_listener, TrustEveryonePolicy)?;

    println!("\n[✓] End-to-end encrypted secure channel was established.\n");

    loop {
        // Read a message from standard input.
        println!("Type a message for Bob's echoer:");
        let mut message = String::new();
        io::stdin().read_line(&mut message).expect("Error reading from stdin.");
        let message = message.trim();

        ctx.send(route![channel.clone(), "echoer"], message.to_string()).await?;

        // Wait to receive a reply and print it.
        let reply = ctx.receive::<String>().await?;
        println!("Alice received an echo: {}\n", reply); // should print "Hello Ockam!"
    }

    // This program will keep running until you stop it with Ctrl-C
}

```
