use ockam::{route, Context, Identity, Result, TrustEveryonePolicy, Vault};
use ockam::{stream::Stream, TcpTransport, Unique, TCP};
use std::io;

#[ockam::node]
async fn main(mut ctx: Context) -> Result<()> {
    // Initialize the TCP Transport.
    TcpTransport::create(&ctx).await?;

    // Create a Vault to safely store secret keys for Alice.
    let vault = Vault::create();

    // Create an Identity to represent Alice.
    let alice = Identity::create(&ctx, &vault).await?;

    // This program expects that Bob has created two streams
    // bob_to_alice and alice_to_bob on the cloud node at 1.node.ockam.network:4000
    // We need the user to provide the addresses of these streams.

    // From standard input, read bob_to_alice stream address.
    println!("\nEnter the bob_to_alice stream address: ");
    let mut b_to_a_stream_address = String::new();
    io::stdin().read_line(&mut b_to_a_stream_address).expect("Error stdin.");
    let b_to_a_stream_address = b_to_a_stream_address.trim();

    // From standard input, read alice_to_bob stream address.
    println!("\nEnter the alice_to_bob stream address: ");
    let mut a_to_b_stream_address = String::new();
    io::stdin().read_line(&mut a_to_b_stream_address).expect("Error stdin.");
    let a_to_b_stream_address = a_to_b_stream_address.trim();

    // We now know that the route to:
    // - send messages to bob is [(TCP, "1.node.ockam.network:4000"), a_to_b_stream_address]
    // - receive messages from bob is [(TCP, "1.node.ockam.network:4000"), b_to_a_stream_address]

    // Starts a sender (producer) for the alice_to_bob stream and a receiver (consumer)
    // for the `bob_to_alice` stream to get two-way communication.

    let node_in_hub = (TCP, "1.node.ockam.network:4000");
    let (sender, _receiver) = Stream::new(&ctx)
        .await?
        .stream_service("stream_kafka")
        .index_service("stream_kafka_index")
        .client_id(Unique::with_prefix("alice"))
        .connect(route![node_in_hub], a_to_b_stream_address, b_to_a_stream_address)
        .await?;

    // As Alice, connect to Bob's secure channel listener using the sender, and
    // perform an Authenticated Key Exchange to establish an encrypted secure
    // channel with Bob.
    let r = route![sender.clone(), "listener"];
    let channel = alice.create_secure_channel(r, TrustEveryonePolicy).await?;

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
