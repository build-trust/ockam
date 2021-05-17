```
title: Routing over many transport hops
```

# Routing over many transport hops

## Responder node

Create a new file at:

```
touch examples/08-routing-over-transport-many-hops-responder.rs
```

Add the following code to this file:

```rust
// examples/08-routing-over-transport-many-hops-responder.rs
// This node starts a tcp listener and an echoer worker.
// It then runs forever waiting for messages.

use ockam::{Context, Result, TcpTransport};
use ockam_get_started::Echoer;

#[ockam::node]
async fn main(ctx: Context) -> Result<()> {
    // Initialize the TCP Transport.
    let tcp = TcpTransport::create(&ctx).await?;

    // Create a TCP listener and wait for incoming connections.
    tcp.listen("127.0.0.1:4000").await?;

    // Create an echoer worker
    ctx.start_worker("echoer", Echoer).await?;

    // Don't call ctx.stop() here so this node runs forever.
    Ok(())
}

```

## Middle node

Create a new file at:

```
touch examples/08-routing-over-transport-many-hops-middle.rs
```

Add the following code to this file:

```rust
// examples/08-routing-over-transport-many-hops-middle.rs
// This node creates a tcp connection to a node at 127.0.0.1:4000
// Starts a tcp listener at 127.0.0.1:3000
// It then runs forever waiting to route messages.

use ockam::{Context, Result, TcpTransport};

#[ockam::node]
async fn main(ctx: Context) -> Result<()> {
    // Initialize the TCP Transport.
    let tcp = TcpTransport::create(&ctx).await?;

    // Create a TCP connection
    tcp.connect("127.0.0.1:4000").await?;

    // Create a TCP listener and wait for incoming connections.
    tcp.listen("127.0.0.1:3000").await?;

    // Don't call ctx.stop() here so this node runs forever.
    Ok(())
}

```

## Initiator node

Create a new file at:

```
touch examples/08-routing-over-transport-many-hops-initiator.rs
```

Add the following code to this file:

```rust
// examples/08-routing-over-transport-many-hops-initiator.rs
// This node routes a message, to a worker on a different node, over two tcp transport hops.

use ockam::{Context, Result, Route, TcpTransport, TCP};

#[ockam::node]
async fn main(mut ctx: Context) -> Result<()> {
    // Initialize the TCP Transport.
    let tcp = TcpTransport::create(&ctx).await?;

    // Create a TCP connection.
    tcp.connect("127.0.0.1:3000").await?;

    // Send a message to the "echoer" worker, on a different node, over two tcp hops.
    ctx.send(
        Route::new()
            .append_t(TCP, "127.0.0.1:3000") // middle node
            .append_t(TCP, "127.0.0.1:4000") // responder node
            .append("echoer"), // echoer worker on responder node
        "Hello Ockam!".to_string(),
    )
    .await?;

    // Wait to receive a reply and print it.
    let reply = ctx.receive::<String>().await?;
    println!("App Received: {}", reply); // should print "Hello Ockam!"

    // Stop all workers, stop the node, cleanup and return.
    ctx.stop().await
}

```

## Run

```
cargo run --example 08-routing-over-transport-many-hops-responder
```

```
cargo run --example 08-routing-over-transport-many-hops-middle
```

```
cargo run --example 08-routing-over-transport-many-hops-initiator
```

<div style="display: none; visibility: hidden;">
<hr><b>Next:</b> <a href="../09-secure-channel-over-many-transport-hops">09. Secure Channel over many transport hops</a>
</div>
