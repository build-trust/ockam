// examples/alice.rs

use ockam::{route, Context, Entity, Result, SecureChannels, TrustEveryonePolicy, Vault};
use ockam::{stream::Stream, TcpTransport, TCP, Unique};
use std::io;

#[ockam::node]
async fn main(mut ctx: Context) -> Result<()> {
    // Initialize the TCP Transport.
    TcpTransport::create(&ctx).await?;

    // Create a Vault to safely store secret keys for Alice.
    let vault = Vault::create(&ctx)?;

    // Create an Entity to represent Alice.
    let mut alice = Entity::create(&ctx, &vault)?;

    // This program expects that Bob has created a bi-directional stream that
    // will relay messages for his secure channel listener, on the Ockam node
    // at 1.node.ockam.network:4000.
    //
    // From standard input, read the bi-directional stream names for
    // Bob's secure channel listener.
    println!("\nEnter the stream sender name for Bob: ");
    let mut sender_name = String::new();
    io::stdin().read_line(&mut sender_name).expect("Error reading from stdin.");
    let sender_name = sender_name.trim();

    println!("\nEnter the stream receiver name for Bob: ");
    let mut receiver_name = String::new();
    io::stdin().read_line(&mut receiver_name).expect("Error reading from stdin.");
    let receiver_name = receiver_name.trim();

    // Use the tcp address of the node to get a route to Bob's secure
    // channel listener via the Kafka stream client.
    let route_to_bob_listener = route![(TCP, "1.node.ockam.network:4000")];
    let (sender, _receiver) = Stream::new(&ctx)?
        .stream_service("stream_kafka")
        .index_service("stream_kafka_index")
        .client_id(Unique::with_prefix("alice"))
        .connect(
            route_to_bob_listener, // route to hub
            receiver_name.clone(), // outgoing stream
            sender_name.clone()    // incoming stream
        )
        .await?;

    // As Alice, connect to Bob's secure channel listener, and perform
    // an Authenticated Key Exchange to establish an encrypted secure
    // channel with Bob.
    let channel = alice.create_secure_channel(route![
        sender.clone(), // via the "alice-to-bob" stream
        "listener"      // to the secure channel "listener"
    ], TrustEveryonePolicy)?;


    println!("\n[✓] End-to-end encrypted secure channel was established.\n");

    loop {
        // Read a message from standard input.
        println!("Type a message for Bob's echoer:");
        let mut message = String::new();
        io::stdin().read_line(&mut message).expect("Error reading from stdin.");
        let message = message.trim();

        // Send the provided message, through the channel, to Bob's echoer.
        ctx.send(
            route![
                channel.clone(), // via the secure channel
                "echoer",
            ],
            message.to_string()
        ).await?;

        // Wait to receive an echo and print it.
        let reply = ctx.receive::<String>().await?;
        println!("Alice received an echo: {}\n", reply); // should print "Hello Ockam!"
    }

    // This program will keep running until you stop it with Ctrl-C
}
