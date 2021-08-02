# End-to-end encrypted messaging with Kafka and Ockam Secure Channels

## Background

When using Apache Kafka and other stream processing services a common concern is data access and security.

Many fields, such as finance and personal information handling, have statutory requirements for data to be encrypted when stored or in transit.

Modern networks and data pipelines transfer data through multiple endpoints, often controlled by multiple vendors. It is therefore not sufficient for encryption to only cover the transit between arbitrary pairs of endpoints or once data comes to rest in storage.

This has led to the emergence of End-to-End encryption as the preferred model for messaging.

There were several attempts to define and implement end-to-end encryption for messaging system like Kafka, for example [KIP-317](https://cwiki.apache.org/confluence/display/KAFKA/KIP-317%3A+Add+end-to-end+data+encryption+functionality+to+Apache+Kafka)

Most of these attempts are using some sort of key storage and exchange persistent keys between devices. A fundamental weakness of such a system is its vulnerability to any party able to acquire the key.

Another approach would be to create a transient encryption key when establishing the session between two ends and have only them access it.

This approach is implemented in [Ockam Secure Channels](../../rust/06-secure-channel)


## Rust Example

Let's build end-to-end protected communication between Alice and Bob, via Apache Kafka using Ockam.

In order to establish a Secure Channel we need to be able to send messages between two ends bidirectionally. For that we are going to use two Kafka topics.

For simplicity we're going to use single partition topics.

Our goals are to make the message exchange:

- secure: no one except the endpoints can decrypt the messages
- reliable: messages are delivered eventually as long as the endpoints are alive

We'll create two small Rust programs called Alice and Bob. We want Bob to create a secure channel listener
and ask Alice to initiate a secure handshake (authenticated key exchange) with this listener. We'll imagine
that Bob and Alice are running on two separate computers and this handshake must happen over the Internet.

We'll also imagine that Bob is running within a private network and cannot open a public port exposed to
the Internet. Instead, Bob registers a bi-directional Kafka stream on an Ockam Node, running as a cloud service in Ockam Hub.

This node is at TCP address `1.node.ockam.network:4000` and offers two Kafka services:
`stream_kafka` and `stream_kafka_index`.

### Setup

If you don't have it, please [install](https://www.rust-lang.org/tools/install) the latest version of Rust.

```
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

Next, create a new cargo project to get started:

```
cargo new --lib hello_ockam && cd hello_ockam && mkdir examples \
  && echo 'ockam = "*"' >> Cargo.toml && cargo build
```

If the above instructions don't work on your machine please
[post a question](https://github.com/ockam-network/ockam/discussions/1642),
we would love to help.


### Bob

Create a file at `examples/bob.rs` and copy the below code snippet to it.

```rust
// examples/bob.rs

use ockam::{Context, Entity, Result, SecureChannels, TrustEveryonePolicy, Vault};
use ockam::{route, stream::Stream, Routed, TcpTransport, Worker, TCP, Unique};

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
    let sender_name = Unique::with_prefix("alice-to-bob");
    let receiver_name = Unique::with_prefix("bob-to-alice");
    Stream::new(&ctx)?
        .stream_service("stream_kafka")
        .index_service("stream_kafka_index")
        .client_id(Unique::with_prefix("bob"))
        .connect(
            route![node_in_hub],  // route to hub
            sender_name.clone(),  // outgoing stream
            receiver_name.clone() // incoming stream
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
```


### Alice

Create a file at `examples/alice.rs` and copy the below code snippet to it.

```rust
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
```


### Run the example

1. Run Bob’s program:

    ```
    cargo run --example bob
    ```

    The Bob program creates a Secure Channel Listener to accept requests to begin an Authenticated
    Key Exchange. It also connects, over TCP, to the cloud node at `1.node.ockam.network:4000` and creates
    a bi-directional Kafka stream on that cloud node. All messages that arrive on that stream will be relayed to
    Bob using the TCP connection that Bob created as a client.

    Bob also starts an Echoer worker that prints any message it receives and echoes it back on its return route.

2. The Bob program will print two stream names, a sender and receiver, which are the stream relay addresses for Bob on the cloud node, copy them.

3. In a separate terminal window, in the same directory path, run the Alice program:

    ```
    cargo run --example alice
    ```

4. It will stop to ask for Bob's stream names that were printed in step 2. Enter them.

    This will tell Alice that the route to reach Bob is via the stream names registered on `(TCP, "1.node.ockam.network:4000")`.

    When Alice sends a message along this route, the Ockam routing layer will look at the first address
    in the route and hand the message to the TCP transport. The TCP transport will connect with the cloud
    node over TCP and hand the message to it.

    The routing layer on the cloud node will then take the message via the Kafka stream to Bob. The
    Kafka client will send the message to Bob over the TCP connection Bob had earlier created with the
    cloud node.

    Replies, from Bob, take the same path back and the entire secure channel handshake is completed is this way.

5. End-to-end Secure Channel is established. Send messages to Bob and get their echoes back.

    Once the secure channel is established, the Alice program will stop and ask you to enter a message for
    Bob. Any message that you enter, is delivered to Bob using the secure channel, via the cloud node. The echoer
    on Bob will echo the messages back on the same path and Alice will print it.

## Conclusion

Congratulations on creating your first end-to-end encrypted application 🥳.

We [discussed](#remove-implicit-trust-in-porous-network-boundaries) that, in order to have a small and manageable
vulnerability surface, distributed applications must use mutually authenticated, end-to-end encrypted channels.
Implementing an end-to-end secure channel protocol, from scratch, is complex, error prone,
and will take more time than application teams can typically dedicate to this problem.

In the above example, we created a mutually authenticated, end-to-end encrypted channel in __75 lines of code__
(excluding comments).

Ockam combines proven cryptographic building blocks into a set of reusable protocols for distributed
applications to communicate security and privately. The above example only scratched the surface of what
is possible with the tools that our included in the `ockam` Rust crate.

To learn more, please see our [step-by-step guide](../../guides/rust#step-by-step).

<div style="display: none; visibility: hidden;">
<hr><b>Next:</b> <a href="../../guides/rust#step-by-step">A step-by-step introduction</a>
</div>



<!--

I don't know if we still want to integrate this?

## What just happened?

The example program is using Ockam framework to establish connection to a cloud node managed by Ockam Hub (../../rust/07-hub) and using the [Ockam Stream API](../../rust/xx-streams) to create topics in Confluent Cloud Kafka cluster.

Using these topics, the two programs then establish an [Ockam Secure Channel](../../rust/06-secure-channel) to exchange messages.

Every message you type is encrypted by Secure Channel, sent to the Hub Node and put in the Kafka topic by the Hub Node.

Finally it's fetched by the other program and decrypted by Secure Channel.

<img src="./kafka-end-to-end.png" width="100%">


The messages printed as `Push to the stream` and `Pull from the stream` are the messages sent to Kafka topics.

Messages in those logs are what the network or Kafka broker will see. As you can see the messages are encrypted and not exposed to the broker.

**NOTE** The encryption key is transient and generated on secure channel establishment. After you exit the example programs all the messages stored in Kafka will be lost as they can no longer be decrypted.


### Message flow

<img src="./sequence.png" width="100%">

## What's next?

- More about the [Ockam framework](../../rust/)
- More about [Secure Channels](../../rust/06-secure-channel)
- More about [Ockam Hub](../../rust/07-hub)
- More about [Ockam Streams and Kafka integration](../../rust/xx-streams)



Running the example:


```
MODE=responder ./ockam_kafka_e2ee

Created kafka topics.

Receiving messages from: generated_abc
Sending messages to: generated_dfe

Use the following arguments to run the other node:

IN=generated_dfe OUT=generated_abc MODE=initiator ./ockam_kafka_e2ee

Waiting for secure channel...

Secure channel established

Received message <encrypted blah>

Secure channel decrypted message: blah

```


```
IN=generated_dfe OUT=generated_abc MODE=initiator ./ockam_kafka_e2ee

Initiated stream
Established secure channel

Secure channel established

Enter your message:

>>> blah

Message sent through secure channel

Secure channel encrypted message: <encrypted blah>


```



-->
