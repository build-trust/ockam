```
title: Secure channel over Streams
```

# Secure channel over Streams

## Introduction

In previous examples we demonstrated sending messages through secure channels as well as sending messages via streams.

Let's now combine these two examples and send messages via streams using secure channels.

This gives us both end-to-end encryption and reliable message delivery with only minimal changes to our code.


## App worker

For this example we'll create two nodes that communicate with each other via the Ockam Hub stream service.

The first node, which we will refer to as the "responder", will be responsible for running the echoer worker and the secure channel listener.

The other node, called the "initiator", will initiate the secure channel protocol and send a message to the echoer worker.

Connecting these two nodes will be the bi-directional stream managed by Ockam Hub.

Encryption is managed by the nodes and ensures that Ockam Hub is unable to inspect the content of any messages.


### Responder node

Create a new file at:

```
touch examples/15-secure-channel-over-stream-over-cloud-node-responder.rs
```

Add the following code to this file:

```rust
use ockam::{route, stream::Stream, Context, Result, SecureChannel, TcpTransport, Vault, TCP};
use ockam_get_started::Echoer;
use std::time::Duration;

#[ockam::node]
async fn main(ctx: Context) -> Result<()> {
    let tcp = TcpTransport::create(&ctx).await?;
    tcp.connect("127.0.0.1:4000").await?;

    // Create a vault
    let vault = Vault::create(&ctx)?;

    // Create a secure channel listener at address "secure_channel_listener"
    SecureChannel::create_listener(&ctx, "secure_channel_listener", &vault).await?;

    // Create a bi-directional stream
    Stream::new(&ctx)?
        .stream_service("stream")
        .index_service("stream_index")
        .client_id("secure-channel-over-stream-over-cloud-node-responder")
        .with_interval(Duration::from_millis(100))
        .connect(
            route![(TCP, "127.0.0.1:4000")],
            // Stream name from THIS node to the OTHER node
            "secure-channel-test-b-a",
            // Stream name from the OTHER node to THIS node
            "secure-channel-test-a-b",
        )
        .await?;

    // Start an echoer worker
    ctx.start_worker("echoer", Echoer).await?;

    // Don't call ctx.stop() here so this node runs forever.
    Ok(())
}
```

### Initiator node

Create a new file at:

```
touch examples/15-secure-channel-over-stream-over-cloud-node-initiator.rs
```

Add the following code to this file:

```rust
use ockam::{
    route, stream::Stream, Context, Result, Route, SecureChannel, TcpTransport, Vault, TCP,
};
use std::time::Duration;

#[ockam::node]
async fn main(mut ctx: Context) -> Result<()> {
    let tcp = TcpTransport::create(&ctx).await?;
    tcp.connect("127.0.0.1:4000").await?;

    // Create a vault
    let vault = Vault::create(&ctx)?;

    // Create a bi-directional stream
    let (tx, _rx) = Stream::new(&ctx)?
        .stream_service("stream")
        .index_service("stream_index")
        .client_id("secure-channel-over-stream-over-cloud-node-initiator")
        .with_interval(Duration::from_millis(100))
        .connect(
            route![(TCP, "127.0.0.1:4000")],
            // Stream name from THIS node to the OTHER node
            "secure-channel-test-a-b",
            // Stream name from the OTHER node to THIS node
            "secure-channel-test-b-a",
        )
        .await?;

    // Create a secure channel via the stream
    let channel = SecureChannel::create(
        &ctx,
        Route::new()
            // Send via the stream
            .append(tx.clone())
            // And then to the secure_channel_listener
            .append("secure_channel_listener"),
        &vault,
    )
    .await?;

    // Send a message through the channel to the "echoer"
    ctx.send(
        Route::new().append(channel.address()).append("echoer"),
        "Hello World!".to_string(),
    )
    .await?;

    // Wait for the reply
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
cargo run --example 15-secure-channel-over-stream-over-cloud-node-responder
```

To start the second node run:

```
cargo run --example 15-secure-channel-over-stream-over-cloud-node-initiator
```

You now should see the log message from the initiator: `Reply through secure channel via stream: ...`

## Message flow

<img src="./sequence.png" width="100%">

<div style="display: none; visibility: hidden;">
</div>
