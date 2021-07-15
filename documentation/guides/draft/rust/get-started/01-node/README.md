```yaml
title: Nodes
```

# Nodes

An Ockam Node is an asynchronous execution environment that can run very
lightweight, concurrent, stateful actors called Ockam Workers. A node can
deliver messages from one worker to another worker. Nodes can also route
messages to workers on other remote nodes.

A node requires an asynchronous runtime to concurrently execute workers.
The default Ockam Node implementation uses Tokio, a popular asynchronous
runtime in the Rust ecosystem. Over time, we plan to support Ockam Node
implementations for various `no_std` embedded targets.

The first thing any Ockam program must do is setup and start an Ockam node.
You could do these steps manually, but for convenience we provide an
`#[ockam::node]` attribute that injects all of this initialization.
It creates the asynchronous environment, initializes worker management,
sets up routing and initializes the node context.

## Create a node

For your new node, create a new file at `examples/01-node.rs` in your
[ockam_get_started](../00-setup) project:

```
mkdir -p examples
touch examples/01-node.rs
```

Add the following code to this file:

```rust
// examples/01-node.rs

use ockam::{Context, Result};

#[ockam::node]
async fn main(mut ctx: Context) -> Result<()> {
    // Stop the node as soon as it starts.
    ctx.stop().await
}
```

Here we add the `#[ockam::node]` attribute to an `async` main function that
receives the node execution context as a parameter and returns `ockam::Result`
which helps make our error reporting better.

As soon as the main function starts, we use `ctx.stop()` to immediately stop
the node that was just started. If we don't add this line, the node will run
forever.

To run the node program:

```
cargo run --example 01-node
```

This will download various dependencies, compile and then run our code. When it
runs, you'll see log output that shows the node starting and then
immediately shutting down.

<div style="display: none; visibility: hidden;">
<hr><b>Next:</b> <a href="../02-worker">02. Create an Ockam worker</a>
</div>
