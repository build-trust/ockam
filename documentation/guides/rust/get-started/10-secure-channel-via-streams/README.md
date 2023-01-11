```
title: Secure channel via Streams
```

# Secure channel via Streams

## Introduction

In previous examples we demonstrated sending messages through secure channels as well as sending messages via streams.

Let's now combine these two examples and send messages via streams using secure channels.

This gives us both end-to-end encryption and reliable message delivery with only minimal changes to our code.

## Service set-up



## App worker

For this example we'll create two nodes that communicate with each other via the Ockam Hub stream service.

The first node, which we will refer to as the "responder", will be responsible for running the echoer worker and the secure channel listener.

The other node, called the "initiator", will initiate the secure channel protocol and send a message to the echoer worker.

Connecting these two nodes will be the bi-directional stream managed by Ockam Hub.

Encryption is managed by the nodes and ensures that Ockam Hub is unable to inspect the content of any messages.

**NOTE:** You will need a Hub Node with Kafka integration for this example. To create a new one, please follow the [Creating Hub Nodes](../07-hub) guide.

### Responder node

Create a new file at:

```
touch examples/10-secure-channel-via-streams-responder.rs
```

Add the following code to this file:

```rust
// examples/10-secure-channel-via-streams-responder.rs
use hello_ockam::Echoer;
use ockam::access_control::AllowAll;
use ockam::authenticated_storage::InMemoryStorage;
use ockam::identity::{Identity, SecureChannelRegistry, TrustEveryonePolicy};
use ockam::{route, stream::Stream, vault::Vault, Context, Result, TcpTransport, TCP};

#[ockam::node]
async fn main(ctx: Context) -> Result<()> {
    let _tcp = TcpTransport::create(&ctx).await?;

    // Set the address of the Kafka node you created here. (e.g. "192.0.2.1:4000")
    let hub_node_tcp_address = "<Your node Address copied from hub.ockam.network>";

    // Create a vault
    let vault = Vault::create();

    // Create an Identity
    let bob = Identity::create(&ctx, &vault).await?;

    // Create a secure channel listener at address "secure_channel_listener"
    bob.create_secure_channel_listener(
        "secure_channel_listener",
        TrustEveryonePolicy,
        &InMemoryStorage::new(),
        &SecureChannelRegistry::new(),
    )
    .await?;

    // Create a stream client
    Stream::new(&ctx)
        .await?
        .stream_service("stream_kafka")
        .index_service("stream_kafka_index")
        .client_id("secure-channel-over-stream-over-cloud-node-responder")
        .connect(
            route![(TCP, hub_node_tcp_address)], // route to hub
            "sc-responder-to-initiator",         // outgoing stream
            "sc-initiator-to-responder",         // incoming stream
        )
        .await?;

    // Start an echoer worker
    ctx.start_worker("echoer", Echoer, AllowAll, AllowAll).await?;

    // Don't call ctx.stop() here so this node runs forever.
    Ok(())
}

```

### Initiator node

Create a new file at:

```
touch examples/10-secure-channel-via-streams-initiator.rs
```

Add the following code to this file:

```rust
// examples/10-secure-channel-via-streams-initiator.rs
use ockam::authenticated_storage::InMemoryStorage;
use ockam::identity::{Identity, SecureChannelRegistry, TrustEveryonePolicy};
use ockam::{route, stream::Stream, vault::Vault, Context, Result, TcpTransport, TCP};

#[ockam::node]
async fn main(mut ctx: Context) -> Result<()> {
    let _tcp = TcpTransport::create(&ctx).await?;

    // Set the address of the Kafka node you created here. (e.g. "192.0.2.1:4000")
    let hub_node_tcp_address = "<Your node Address copied from hub.ockam.network>";

    // Create a vault
    let vault = Vault::create();

    // Create an Identity
    let alice = Identity::create(&ctx, &vault).await?;

    // Create a stream client
    let (sender, _receiver) = Stream::new(&ctx)
        .await?
        .stream_service("stream_kafka")
        .index_service("stream_kafka_index")
        .client_id("secure-channel-over-stream-over-cloud-node-initiator")
        .connect(
            route![(TCP, hub_node_tcp_address)], // route to hub
            "sc-initiator-to-responder",         // outgoing stream
            "sc-responder-to-initiator",         // incoming stream
        )
        .await?;

    // Create a secure channel
    let secure_channel = alice
        .create_secure_channel(
            route![
                sender.clone(),            // via the "sc-initiator-to-responder" stream
                "secure_channel_listener"  // to the "secure_channel_listener" listener
            ],
            TrustEveryonePolicy,
            &InMemoryStorage::new(),
            &SecureChannelRegistry::new(),
        )
        .await?;

    // Send a message
    ctx.send(
        route![
            secure_channel.address(), // via the secure channel
            "echoer"                  // to the "echoer" worker
        ],
        "Hello World!".to_string(),
    )
    .await?;

    // Receive a message from the "sc-responder-to-initiator" stream
    let reply = ctx.receive_block::<String>().await?;
    println!("Reply through secure channel via stream: {}", reply);

    ctx.stop().await
}

```

This code starts a stream client, creates a bi-directional stream and then establishes a secure channel between the client and the stream address.

Messages can now be sent normally via the stream through the secure channel.


### Run

To start the first node run:

```
cargo run --example 10-secure-channel-via-streams-responder
```

To start the second node run:

```
cargo run --example 10-secure-channel-via-streams-initiator
```

You now should see the log message from the initiator: `Reply through secure channel via stream: ...`

## Message flow

<img src="./sequence.png" width="100%">

<div style="display: none; visibility: hidden;">
</div>
