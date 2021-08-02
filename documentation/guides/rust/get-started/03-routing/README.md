```
title: Routing
```

# Routing

So far, we've [created an `"echoer"` worker](../02-worker#readme) on our node,
sent it a message, and received a reply.

This worker was a simple one hop away from our `"app"` worker. Ockam's
application layer routing protocols allows us to send messages over multiple
hops, within one node, or across many nodes.

To achieve this, messages carry with them two meta fields: `onward_route`
and `return_route`, where a route is a list of addresses.

To get a sense of how that works, let's route a message over two hops.

## Hop worker

For demonstration, we'll create a simple worker, called `Hop`, that takes
every incoming message and forwards it to the next address in
the `onward_route` of that message.

Just before forwarding the message, `Hop`'s handle message function will:

1. Print the message
1. Remove its own address (first address) from the `onward_route`, by calling `step()`
1. Insert its own address as the first address in the `return_route` by calling `prepend()`

Create a new file at:

```
touch src/hop.rs
```

Add the following code to this file:

```rust
// src/hop.rs

use ockam::{Any, Context, Result, Routed, Worker};

pub struct Hop;

#[ockam::worker]
impl Worker for Hop {
    type Context = Context;
    type Message = Any;

    /// This handle function takes any incoming message and forwards
    /// it to the next hop in it's onward route
    async fn handle_message(&mut self, ctx: &mut Context, msg: Routed<Any>) -> Result<()> {
        println!("Address: {}, Received: {}", ctx.address(), msg);

        // Some type conversion
        let mut message = msg.into_local_message();
        let transport_message = message.transport_mut();

        // Remove my address from the onward_route
        transport_message.onward_route.step()?;

        // Insert my address at the beginning return_route
        transport_message.return_route.modify().prepend(ctx.address());

        // Send the message on its onward_route
        ctx.forward(message).await
    }
}

```

To make this `Hop` type accessible to our main program, export it
from `src/lib.rs` by adding the following to it:

```rust
mod hop;
pub use hop::*;
```

## Echoer worker

We'll also use the `Echoer` worker that we created in the
[previous example](../02-worker#echoer-worker). So make sure that it stays
exported from `src/lib.rs`

## App worker

Next, let's create our main `"app"` worker.

In the code below we start an `Echoer` worker at address `"echoer"` and a `Hop`
worker at address `"h1"`. Then, we send a message along the `h1 => echoer`
route by passing `route!["h1", "echoer"]` to `send(..)`.

Create a new file at:

```
touch examples/03-routing.rs
```

Add the following code to this file:

```rust
// examples/03-routing.rs
// This node routes a message.

use hello_ockam::{Echoer, Hop};
use ockam::{route, Context, Result};

#[ockam::node]
async fn main(mut ctx: Context) -> Result<()> {
    // Start a worker, of type Echoer, at address "echoer"
    ctx.start_worker("echoer", Echoer).await?;

    // Start a worker, of type Hop, at address "h1"
    ctx.start_worker("h1", Hop).await?;

    // Send a message to the worker at address "echoer",
    // via the worker at address "h1"
    ctx.send(route!["h1", "echoer"], "Hello Ockam!".to_string()).await?;

    // Wait to receive a reply and print it.
    let reply = ctx.receive::<String>().await?;
    println!("App Received: {}", reply); // should print "Hello Ockam!"

    // Stop all workers, stop the node, cleanup and return.
    ctx.stop().await
}

```

To run this new node program:

```
cargo run --example 03-routing
```

Note the message flow and how routing information is manipulated as the message
travels.

<img src="./sequence.png" width="100%">

## Routing over many hops

Routing is not limited to [one](../02-worker#readme) or [two hops](#app-worker),
we can easily create routes with many hops. Let's try that in a quick example:

This time we'll create multiple hop workers between the `"app"` and
the `"echoer"` and route our message through them.

Create a new file at:

```
touch examples/03-routing-many-hops.rs
```

Add the following code to this file:

```rust
// examples/03-routing-many-hops.rs
// This node routes a message through many hops.

use hello_ockam::{Echoer, Hop};
use ockam::{route, Context, Result};

#[ockam::node]
async fn main(mut ctx: Context) -> Result<()> {
    // Start an Echoer worker at address "echoer"
    ctx.start_worker("echoer", Echoer).await?;

    // Start 3 hop workers at addresses "h1", "h2" and "h3".
    ctx.start_worker("h1", Hop).await?;
    ctx.start_worker("h2", Hop).await?;
    ctx.start_worker("h3", Hop).await?;

    // Send a message to the echoer worker via the "h1", "h2", and "h3" workers
    let r = route!["h1", "h2", "h3", "echoer"];
    ctx.send(r, "Hello Ockam!".to_string()).await?;

    // Wait to receive a reply and print it.
    let reply = ctx.receive::<String>().await?;
    println!("App Received: {}", reply); // should print "Hello Ockam!"

    // Stop all workers, stop the node, cleanup and return.
    ctx.stop().await
}

```

To run this new node program:

```
cargo run --example 03-routing-many-hops
```

Note the message flow.

<img src="./sequence-many-hops.png" width="100%">

<div style="display: none; visibility: hidden;">
<hr><b>Next:</b> <a href="../04-transport#readme">04. Transport</a>
</div>
