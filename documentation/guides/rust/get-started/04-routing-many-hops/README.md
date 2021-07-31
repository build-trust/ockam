```
title: Routing over many hops
```

# Routing over many hops

Routing is not limited to [one](../02-worker) or [two hops](../03-routing),
we can easily create routes with many hops. Let's try that in a quick example:

## App worker

This time we'll create multiple hop workers between the `"app"` and
the `"echoer"` and route our message through them.

Create a new file at:

```
touch examples/04-routing-many-hops.rs
```

Add the following code to this file:

```rust
// examples/04-routing-many-hops.rs
// This node routes a message through many hops.

use ockam::{route, Context, Result};
use ockam_get_started::{Echoer, Hop};

#[ockam::node]
async fn main(mut ctx: Context) -> Result<()> {
    // Start an Echoer worker at address "echoer"
    ctx.start_worker("echoer", Echoer).await?;

    // Start 3 hop workers at addresses "h1", "h2" and "h3".
    ctx.start_worker("h1", Hop).await?;
    ctx.start_worker("h2", Hop).await?;
    ctx.start_worker("h3", Hop).await?;

    // Send a message to the echoer worker via the "h1", "h2", and "h3" workers
    ctx.send(
        // route to the "echoer" worker via "h1", "h2" and "h3"
        route!["h1", "h2", "h3", "echoer"],
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

To run this new node program:

```
cargo run --example 04-routing-many-hops
```

Note the message flow.

## Message Flow

<img src="./sequence.png" width="100%">

<div style="display: none; visibility: hidden;">
<hr><b>Next:</b> <a href="../05-secure-channel">05. Secure Channel</a>
</div>
