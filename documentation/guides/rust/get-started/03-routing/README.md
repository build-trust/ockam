```
title: Routing
```

# Routing

So far, we've [created an `"echoer"` worker](../02-worker) on our node, sent
it a message, and received a reply.

This worker was a simple one hop away from our `"app"` worker. Ockam's
application layer routing protocols allows us to send messages over multiple
hops, within one node, or across many nodes.

To achieve this, messages carry with them two meta fields: `onward_route`
and `return_route`, where a route is a list of addresses.

To get a sense of how that works, let's route a message over two hops.

## Hop worker

For demonstration, we'll create a simple middleware worker, called `Hop`, that
takes every incoming message and forwards it to the next address in
the `onward_route` of that message.

Just before forwarding the message, `Hop`'s handle message function will:

1. Print the message
1. Remove its own address (first address) from the `onward_route` by calling `step()`
1. Insert its own address into the `return_route` by calling `prepend()`

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
    type Message = Any;
    type Context = Context;

    /// This handle function takes any incoming message and forwards
    /// it to the next hop in its onward route
    async fn handle_message(&mut self, ctx: &mut Context, msg: Routed<Any>) -> Result<()> {
        println!("Address: {}, Received: {}", ctx.address(), msg);

        let mut msg = msg.into_transport_message();
        msg.onward_route.step()?;
        msg.return_route.modify().prepend(ctx.address());
        ctx.forward(msg).await
    }
}
```

To make this `Hop` type accessible to our main program, export it
from `src/lib.rs` by adding the following to it:

```rust
// src/lib.rs

mod hop;
pub use hop::*;
```

## Echoer worker

We'll also use the `Echoer` worker that we created in the
[previous example](../02-worker#echoer-worker). So make sure that it stays
exported from `src/lib.rs`

## App worker

Next, let's create our main `"app"` worker.

In the code below we start an `Echoer` at address `"echoer"` and a `Hop`
worker at address `"hop1"`. This is familiar from our previous example.

Then, we send a message along the `hop1 => echoer` route by passing
`Route::new().append("hop1").append("echoer")` to `send(..)`.

Create a new file at:

```
touch examples/03-routing.rs
```

Add the following code to this file:

```rust
// examples/03-routing.rs

use ockam::{Context, Result, Route};
use ockam_get_started::{Echoer, Hop};

#[ockam::node]
async fn main(mut ctx: Context) -> Result<()> {
    // Start an Echoer worker at address "echoer"
    ctx.start_worker("echoer", Echoer).await?;

    // Start a Hop worker at address "hop1"
    ctx.start_worker("hop1", Hop).await?;

    // Send a message to the echoer worker via the hop1 worker
    ctx.send(
        Route::new().append("hop1").append("echoer"),
        "Hello Ockam!".to_string(),
    )
    .await?;

    // Wait to receive a reply and print it.
    let reply = ctx.receive::<String>().await?;
    println!("App Received: {}", reply); // should print "Hello Ockam!"

    ctx.stop().await
}
```

To run this new node program:

```
cargo run --example 03-routing
```

Note the message flow and how routing information is manipulated as the message
travels.

## Message Flow

<img src="./sequence.png" width="100%">

<div style="display: none; visibility: hidden;">
<hr><b>Next:</b> <a href="../04-routing-many-hops">04. Routing over many hops</a>
</div>
