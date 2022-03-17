use ockam::{route, Context, Identity, Result, TrustEveryonePolicy, Vault};
use ockam::{stream::Stream, Routed, TcpTransport, Unique, Worker, TCP};

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
    let vault = Vault::create();

    // Create an Identity to represent Bob.
    let bob = Identity::create(&ctx, &vault).await?;

    // Create a secure channel listener for Bob that will wait for requests to
    // initiate an Authenticated Key Exchange.
    bob.create_secure_channel_listener("listener", TrustEveryonePolicy)
        .await?;

    // Connect, over TCP, to the cloud node at `1.node.ockam.network:4000` and
    // request the `stream_kafka` service to create two Kafka backed streams -
    // `alice_to_bob` and `bob_to_alice`.
    //
    // After the streams are created, create:
    // - a receiver (consumer) for the `alice_to_bob` stream
    // - a sender (producer) for the `bob_to_alice` stream.

    let node_in_hub = (TCP, "1.node.ockam.network:4000");
    let b_to_a_stream_address = Unique::with_prefix("bob_to_alice");
    let a_to_b_stream_address = Unique::with_prefix("alice_to_bob");

    Stream::new(&ctx)
        .await?
        .stream_service("stream_kafka")
        .index_service("stream_kafka_index")
        .client_id(Unique::with_prefix("bob"))
        .connect(
            route![node_in_hub],
            b_to_a_stream_address.clone(),
            a_to_b_stream_address.clone(),
        )
        .await?;

    println!("\n[✓] Streams were created on the node at: 1.node.ockam.network:4000");
    println!("\nbob_to_alice stream address is: {}", b_to_a_stream_address);
    println!("alice_to_bob stream address is: {}\n", a_to_b_stream_address);

    // Start a worker, of type Echoer, at address "echoer".
    // This worker will echo back every message it receives, along its return route.
    ctx.start_worker("echoer", Echoer).await?;

    // We won't call ctx.stop() here, this program will run until you stop it with Ctrl-C
    Ok(())
}
