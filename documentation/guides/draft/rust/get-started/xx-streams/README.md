```
title: Reliable message delivery with Streams
```

# Reliable message delivery with Streams

## Introduction

In the previous guides we were sending messages to remote workers without any delivery guarantees.

The workers were sending messages hoping that receiving end would receive them. This messaging mode is usually referred to as: *"at most once delivery"*

In real life, distributed systems are constantly experiencing network interruptions, while workers and devices themselves may crash and restart.

Message brokers solve this problem by introducing message buffers or logs. By maintaining a record of messages sent, the broker can retry delivery in the event of failures.

Ockam Hub integrates with message brokers through the use of Ockam Streams and applications use the Ockam Streams Protocol to communicate.

Further information can be found in the Ockam Stream service protocol definition: https://github.com/ockam-network/proposals/tree/main/design/0009-stream-protocol


### The Stream client API

The Stream Client API is used to configure and initiate a connection to a Stream service and implement Ockam Streams support for your application.

#### Client Configuration

Stream client configuration is facilitated by a [builder pattern](https://doc.rust-lang.org/1.0.0/style/ownership/builders.html) that exposes configuration methods for the stream.

For example:

```rust
let stream = Stream::new(&ctx)?
    .stream_service("stream")
    .index_service("stream_index")
    .client_id("stream-over-cloud-node-responder")
    .with_interval(Duration::from_millis(100));
```

Here the `stream_service()` and `index_service()` methods configure the client to use the basic stream service exposed by Ockam Hub.

The `client_id()` method configures a name for our node that the Stream Service or any other clients can use to uniquely identify this node.

Finally, the `with_interval()` method configures the rate at which nodes poll the stream service for new messages.

#### Client Connection

Once configured, a connection can be made to the stream service with the `connect()` method.

For example:

```rust
let (tx, rx) = stream.connect(
    route![(TCP, "127.0.0.1:4000")], // route
    "stream-a-to-b",                 // outgoing stream
    "stream-b-to-a",                 // incoming stream
).await?;
```

The route parameter describes an Ockam Route to the stream service.

The outgoing and incoming stream parameters refer to the names of the streams we are sending and receiving messages on.

Finally, the `connect()` method returns two routes: `tx` and `rx` that can be used to send and receive messages in the same way as any other transport.


#### Stream Communication

When we have two stream clients running on different nodes with symmetrical stream names configured, the nodes will be able to exchange messages the same way as they would using transports or secure channels.

For example:

```rust
ctx.send(
    tx.to_route().append("echoer"), // route via stream's 'tx' route
    "Hello World!".to_string()      // message
).await?;
```

Here the route parameter describes the `tx` route from stream with the `echoer` worker on the destination node appended to it.


## App worker

In this example we'll set up a bi-directional stream and use it for communication between two nodes.

As in the previous examples, we will create a responder node and initiator node.

The responder node will have an `"echoer"` worker and the initiator node will send it a message through Ockam Hub's stream service.

### Responder node

Create a new file at:

```
touch examples/14-stream-over-cloud-node-responder.rs
```

Add the following code to this file:

```rust
use ockam::{route, stream::Stream, Context, Result, route, TcpTransport, TCP};
use ockam_get_started::Echoer;
use std::time::Duration;

#[ockam::node]
async fn main(ctx: Context) -> Result<()> {
    let tcp = TcpTransport::create(&ctx).await?;
    tcp.connect("127.0.0.1:4000").await?;

    // Start a printer
    ctx.start_worker("echoer", Echoer).await?;

    // Create the stream
    Stream::new(&ctx)?
        .stream_service("stream")
        .index_service("stream_index")
        .client_id("stream-over-cloud-node-initiator")
        .with_interval(Duration::from_millis(100))
        .connect(
            route![(TCP, "127.0.0.1:4000")],
            // Stream name from THIS to OTHER
            "test-b-a",
            // Stream name from OTHER to THIS
            "test-a-b",
        )
        .await?;
    Ok(())
}
```

This code creates a stream client on the Hub node at `127.0.0.1:4000` and starts an echoer worker.

### Initiator node

Create a new file at:

```
touch examples/14-stream-over-cloud-node-initiator.rs
```

Add the following code to this file:

```rust
use ockam::{route, stream::Stream, Context, Result, Route, TcpTransport, TCP};
use std::time::Duration;

#[ockam::node]
async fn main(mut ctx: Context) -> Result<()> {
    let tcp = TcpTransport::create(&ctx).await?;
    tcp.connect("127.0.0.1:4000").await?;

    let (tx, _rx) = Stream::new(&ctx)?
        .stream_service("stream")
        .index_service("stream_index")
        .client_id("stream-over-cloud-node-initiator")
        .with_interval(Duration::from_millis(100))
        .connect(
            route![(TCP, "127.0.0.1:4000")],
            // Stream name from THIS node to the OTHER node
            "test-a-b",
            // Stream name from OTHER to THIS
            "test-b-a",
        )
        .await?;

    ctx.send(tx.to_route().append("echoer"), "Hello World!".to_string())
        .await?;

    let reply = ctx.receive_block::<String>().await?;
    println!("Reply via stream: {}", reply);

    ctx.stop().await
}
```

This code creates a stream client, sends a message to the echoer through this client and expects a response.


### Run

You can run initiator and responder in any order because they use stream storage to deliver messages.

To demonstrate this, let's run the initiator first this time:

```
cargo run --example 14-stream-over-cloud-node-initiator
```

Only then do we start the responder:

```
cargo run --example 14-stream-over-cloud-node-responder
```

On the initiator side you should now see the `Reply via stream: ...` message.

## Message flow

<img src="./sequence.png" width="100%">

<div style="display: none; visibility: hidden;">
</div>
