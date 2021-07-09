// examples/bob.rs

use ockam::{route, stream::Stream, Routed, TcpTransport, Unique, Worker, TCP};
use ockam::{Context, Entity, Result, SecureChannels, TrustEveryonePolicy, Vault};

struct Echoer;

// Define an Echoer worker that prints any message it receives and
// echoes it back on its return route.
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
    let vault = Vault::create(&ctx)?;

    // Create an Entity to represent Bob.
    let mut bob = Entity::create(&ctx, &vault)?;

    // Create a secure channel listener for Bob that will wait for requests to
    // initiate an Authenticated Key Exchange.
    bob.create_secure_channel_listener("listener", TrustEveryonePolicy)?;

    // The computer running this program is likely within a private network and not
    // accessible over the internet.
    //
    // To allow Alice and others to initiate an end-to-end secure channel with this program
    // we connect to 1.node.ockam.network:4000 as a TCP client and ask the Kafka streaming
    // service on that node to create a bi-directional stream for us.
    //
    // All messages sent to and arriving at the stream will be relayed
    // using the TCP connection we created as a client.
    let node_in_hub = (TCP, "1.node.ockam.network:4000");
    let sender_name = Unique::with_prefix("bob-to-alice");
    let receiver_name = Unique::with_prefix("alice-to-bob");
    Stream::new(&ctx)?
        .stream_service("stream_kafka")
        .index_service("stream_kafka_index")
        .client_id(Unique::with_prefix("bob"))
        .connect(
            route![node_in_hub],   // route to hub
            sender_name.clone(),   // outgoing stream
            receiver_name.clone(), // incoming stream
        )
        .await?;
    println!("\n[✓] Stream client was created on the node at: 1.node.ockam.network:4000");
    println!("\nStream sender name is: {}", sender_name);
    println!("Stream receiver name is: {}\n", receiver_name);

    // Start a worker, of type Echoer, at address "echoer".
    // This worker will echo back every message it receives, along its return route.
    ctx.start_worker("echoer", Echoer).await?;

    // We won't call ctx.stop() here, this program will run until you stop it with Ctrl-C
    Ok(())
}
