```
title: Routing over a transport
```

# Routing over a transport

## Responder node

Create a new file at:

```
touch examples/07-routing-over-transport-responder.rs
```

Add the following code to this file:

```rust
// examples/07-routing-over-transport-responder.rs
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

## Initiator node

Create a new file at:

```
touch examples/07-routing-over-transport-initiator.rs
```

Add the following code to this file:

```rust
// examples/07-routing-over-transport-initiator.rs
// This node routes a message, to a worker on a different node, over the tcp transport.

use ockam::{Context, Result, Route, TcpTransport, TCP};

#[ockam::node]
async fn main(mut ctx: Context) -> Result<()> {
    // Initialize the TCP Transport.
    let tcp = TcpTransport::create(&ctx).await?;

    // Create a TCP connection.
    tcp.connect("127.0.0.1:4000").await?;

    // Send a message to the "echoer" worker, on a different node, over a tcp transport.
    ctx.send(
        // route to the "echoer" worker, via a tcp connection.
        Route::new()
            .append_t(TCP, "127.0.0.1:4000")
            .append("echoer"),
        // the message you want echo-ed back
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
cargo run --example 07-routing-over-transport-responder
```

```
cargo run --example 07-routing-over-transport-initiator
```

## Message Flow

<img src="./sequence.png" width="100%">

<div style="display: none; visibility: hidden;">
<hr><b>Next:</b> <a href="../08-routing-over-many-transport-hops">08. Routing over many transport hops</a>
</div>
