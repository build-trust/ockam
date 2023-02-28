```
title: Transport
```

# Transport

An Ockam Transport is a plugin for Ockam Routing. It moves Ockam Routing messages
using a specific transport protocol like TCP, UDP, WebSockets, Bluetooth etc.

In previous examples, we routed messages locally within one node. Routing messages
over transport layer connections looks very similar.

Let's try the TcpTransport, we'll need to create two nodes:
a [responder](#responder-node) and an [initiator](#initiator-node).

### Responder node

Create a new file at:

```
touch examples/04-routing-over-transport-responder.rs
```

Add the following code to this file:

```rust
// examples/04-routing-over-transport-responder.rs
// This node starts a tcp listener and an echoer worker.
// It then runs forever waiting for messages.

use hello_ockam::Echoer;
use ockam::access_control::AllowAll;
use ockam::{Context, Result, TcpTransport};

#[ockam::node]
async fn main(ctx: Context) -> Result<()> {
    // Initialize the TCP Transport.
    let tcp = TcpTransport::create(&ctx).await?;

    // Create an echoer worker
    ctx.start_worker("echoer", Echoer, AllowAll, AllowAll).await?;

    // Create a TCP listener and wait for incoming connections.
    tcp.listen("127.0.0.1:4000").await?;

    // Don't call ctx.stop() here so this node runs forever.
    Ok(())
}

```

### Initiator node

Create a new file at:

```
touch examples/04-routing-over-transport-initiator.rs
```

Add the following code to this file:

```rust
// examples/04-routing-over-transport-initiator.rs
// This node routes a message, to a worker on a different node, over the tcp transport.

use ockam::{route, Context, Result, TcpTransport};

#[ockam::node]
async fn main(mut ctx: Context) -> Result<()> {
    // Initialize the TCP Transport.
    let tcp = TcpTransport::create(&ctx).await?;

    // Create a TCP connection to a different node.
    let connection_to_responder = tcp.connect("localhost:4000").await?;

    // Send a message to the "echoer" worker on a different node, over a tcp transport.
    let r = route![connection_to_responder, "echoer"];
    ctx.send(r, "Hello Ockam!".to_string()).await?;

    // Wait to receive a reply and print it.
    let reply = ctx.receive::<String>().await?;
    println!("App Received: {}", reply); // should print "Hello Ockam!"

    // Stop all workers, stop the node, cleanup and return.
    ctx.stop().await
}

```

### Run

Run the responder:

```
cargo run --example 04-routing-over-transport-responder
```

Run the initiator:

```
cargo run --example 04-routing-over-transport-initiator
```

Note the message flow.

<img src="./sequence.png" width="100%">


## Routing over two transport hops

### Responder node

Create a new file at:

```
touch examples/04-routing-over-transport-two-hops-responder.rs
```

Add the following code to this file:

```rust
// examples/04-routing-over-transport-two-hops-responder.rs
// This node starts a tcp listener and an echoer worker.
// It then runs forever waiting for messages.

use hello_ockam::Echoer;
use ockam::access_control::AllowAll;
use ockam::{Context, Result, TcpTransport};

#[ockam::node]
async fn main(ctx: Context) -> Result<()> {
    // Initialize the TCP Transport.
    let tcp = TcpTransport::create(&ctx).await?;

    // Create an echoer worker
    ctx.start_worker("echoer", Echoer, AllowAll, AllowAll).await?;

    // Create a TCP listener and wait for incoming connections.
    tcp.listen("127.0.0.1:4000").await?;

    // Don't call ctx.stop() here so this node runs forever.
    Ok(())
}

```

### Middle node

Create a new file at:

```
touch examples/04-routing-over-transport-two-hops-middle.rs
```

Add the following code to this file:

```rust
// examples/04-routing-over-transport-two-hops-middle.rs
// This node creates a tcp connection to a node at 127.0.0.1:4000
// Starts a tcp listener at 127.0.0.1:3000
// It then runs forever waiting to route messages.

use hello_ockam::Forwarder;
use ockam::access_control::AllowAll;
use ockam::{Context, Result, TcpTransport};

#[ockam::node]
async fn main(ctx: Context) -> Result<()> {
    // Initialize the TCP Transport.
    let tcp = TcpTransport::create(&ctx).await?;

    // Create a TCP connection to the responder node.
    let connection_to_responder = tcp.connect("127.0.0.1:4000").await?;

    // Create a Forwarder worker
    ctx.start_worker(
        "forward_to_responder",
        Forwarder(connection_to_responder),
        AllowAll,
        AllowAll,
    )
    .await?;

    // Create a TCP listener and wait for incoming connections.
    tcp.listen("127.0.0.1:3000").await?;

    // Don't call ctx.stop() here so this node runs forever.
    Ok(())
}

```

### Initiator node

Create a new file at:

```
touch examples/04-routing-over-transport-two-hops-initiator.rs
```

Add the following code to this file:

```rust
// examples/04-routing-over-transport-two-hops-initiator.rs
// This node routes a message, to a worker on a different node, over two tcp transport hops.

use ockam::{route, Context, Result, TcpTransport};

#[ockam::node]
async fn main(mut ctx: Context) -> Result<()> {
    // Initialize the TCP Transport.
    let tcp = TcpTransport::create(&ctx).await?;

    // Create a TCP connection to the middle node.
    let connection_to_middle_node = tcp.connect("localhost:3000").await?;

    // Send a message to the "echoer" worker, on a different node, over two tcp hops.
    let r = route![connection_to_middle_node, "forward_to_responder", "echoer"];
    ctx.send(r, "Hello Ockam!".to_string()).await?;

    // Wait to receive a reply and print it.
    let reply = ctx.receive::<String>().await?;
    println!("App Received: {}", reply); // should print "Hello Ockam!"

    // Stop all workers, stop the node, cleanup and return.
    ctx.stop().await
}

```

## Run

Run the responder:

```
cargo run --example 04-routing-over-transport-two-hops-responder
```

Run middle:

```
cargo run --example 04-routing-over-transport-two-hops-middle
```

Run the initiator:

```
cargo run --example 04-routing-over-transport-two-hops-initiator
```

Note how the message is routed.

<div style="display: none; visibility: hidden;">
<hr><b>Next:</b> <a href="../05-secure-channel#readme">05. Secure Channel</a>
</div>
