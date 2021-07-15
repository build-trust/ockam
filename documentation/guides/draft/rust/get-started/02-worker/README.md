```yaml
title: Workers
```
# Worker

Ockam Workers are lightweight, concurrent, stateful actors.

Workers:
* Run in an Ockam Node.
* Have an application-defined address (like a postal mail or email address).
* Can maintain internal state.
* Can start other new workers.
* Can handle messages from other workers running on the same or a different node.
* Can send messages to other workers running on the same or a different node.

Now that we've [created our first node](../01-node), let's create a new worker,
send it a message, and receive a reply.

## Echoer worker

To create a worker, we create a struct that can optionally have some fields
to store the worker's internal state. If the worker is stateless, it can be
defined as a field-less unit struct.

This struct:
* Must implement the `ockam::Worker` trait.
* Must have the `#[ockam::worker]` attribute on the Worker trait implementation
* Must define two associated types `Context` and `Message`
  * The `Context` type is usually set to `ockam::Context` which is provided by the node implementation.
  * The `Message` type must be set to the type of message the worker wishes to handle.

For a new `Echoer` worker, create a new file at `src/echoer.rs` in your
[ockam_get_started](../00-setup) project. We're creating this inside the `src`
directory so we can easily reuse the `Echoer` in other examples that we'll
write later in this guide:

```
touch src/echoer.rs
```

Add the following code to this file:

```rust
// src/echoer.rs

use ockam::{Context, Result, Routed, Worker};

pub struct Echoer;

#[ockam::worker]
impl Worker for Echoer {
    type Context = Context;
    type Message = String;

    async fn handle_message(&mut self, ctx: &mut Context, msg: Routed<String>) -> Result<()> {
        println!("Address: {}, Received: {}", ctx.address(), msg);

        // Echo the message body back on its return_route.
        ctx.send(msg.return_route(), msg.body()).await
    }
}
```

Note that we define the `Message` associated type of the worker as `String`,
which specifies that this worker expects to handle `String` messages. We then
go on to define a `handle_message(..)` function that will be called whenever
a new message arrives for this worker.

In the Echoer's `handle_message(..)`, we print any incoming message, along
with the address of the `Echoer`. We then take the body of the incoming
message and echo it back on its return route (more about routes soon).

To make this Echoer type accessible to our main program, export it
from `src/lib.rs` file by adding the following to it:

```rust
// src/lib.rs

mod echoer;
pub use echoer::*;
```

## App worker

When a new node starts and calls an `async` main function, it turns that
function into a worker with address of `"app"`. This makes it easy to send and
receive messages from the main function (i.e the `"app"` worker).

In the code below, we start a new `Echoer` worker at address `"echoer"`, send
this `"echoer"` a message `"Hello Ockam!"` and then wait to receive a `String`
reply back from the `"echoer"`.

Create a new file at:

```
touch examples/02-worker.rs
```

Add the following code to this file:

```rust
// examples/02-worker.rs

use ockam::{Context, Result};
use ockam_get_started::Echoer;

#[ockam::node]
async fn main(mut ctx: Context) -> Result<()> {
    // Start an Echoer worker at address "echoer"
    ctx.start_worker("echoer", Echoer).await?;

    // Send a message to the "echoer" worker.
    ctx.send("echoer", "Hello Ockam!".to_string()).await?;

    // Wait to receive a reply and print it.
    let reply = ctx.receive::<String>().await?;
    println!("App Received: {}", reply); // should print "Hello Ockam!"

    ctx.stop().await
}
```

To run this new node program:

```
cargo run --example 02-worker
```

You'll see console output that shows `"Hello Ockam!"` received by the
`"echoer"` and then an echo of it received by the `"app"`.

## Message Flow

<img src="./sequence.png" width="100%">

<div style="display: none; visibility: hidden;">
<hr><b>Next:</b> <a href="../03-routing">03. Routing</a>
</div>
