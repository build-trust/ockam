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

use ockam::{Context, Result};
use ockam_get_started::Echoer;
use ockam_transport_tcp::TcpTransport;

#[ockam::node]
async fn main(ctx: Context) -> Result<()> {
    let tcp = TcpTransport::create(&ctx).await?;
    tcp.listen("127.0.0.1:4000").await?;

    // Create an echoer worker
    ctx.start_worker("echoer", Echoer).await?;

    // This node never shuts down.
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

use ockam::{Context, Result, Route};
use ockam_transport_tcp::{TcpTransport, TCP};

#[ockam::node]
async fn main(mut ctx: Context) -> Result<()> {
    let tcp = TcpTransport::create(&ctx).await?;
    tcp.connect("127.0.0.1:4000").await?;

    // Send a message to the echoer worker, on a different node, over a tcp transport
    ctx.send(
        Route::new()
            .append_t(TCP, "127.0.0.1:4000")
            .append("echoer"),
        "Hello Ockam!".to_string()
    ).await?;

    // Wait to receive a reply and print it.
    let reply = ctx.receive::<String>().await?;
    println!("Initiator Received: {}", reply); // should print "Hello Ockam!"

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
