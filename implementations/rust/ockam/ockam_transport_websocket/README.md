# ockam_transport_websocket

[![crate][crate-image]][crate-link]
[![docs][docs-image]][docs-link]
[![license][license-image]][license-link]
[![discuss][discuss-image]][discuss-link]

Ockam is a library for building devices that communicate securely, privately
and trustfully with cloud services and other devices.

This crate provides a WebSocket Transport for Ockam's Routing Protocol.

This crate requires the rust standard library `"std"`.

We need to define the behavior of the worker that will be processing incoming messages.

```rust
use ockam_core::{Worker, Result, Routed, async_trait};
use ockam_node::Context;

struct MyWorker;

#[async_trait]
impl Worker for MyWorker {
    type Context = Context;
    type Message = String;

    async fn handle_message(&mut self, _ctx: &mut Context, _msg: Routed<String>) -> Result<()> {
        // ...
        Ok(())
    }
}

// Now we can write the main function that will run the previous worker. In this case, our worker will be listening for new connections on port 8000 until the process is manually killed.

use ockam_transport_websocket::WebSocketTransport;
use ockam_node::NodeBuilder;
use ockam_macros::node;

#[ockam_macros::node(crate = "ockam_node")]
async fn main(mut ctx: Context) -> Result<()> {//!
    let ws = WebSocketTransport::create(&ctx).await?;
    ws.listen("localhost:8000").await?; // Listen on port 8000

    // Start a worker, of type MyWorker, at address "my_worker"
    ctx.start_worker("my_worker", MyWorker).await?;

    // Run worker indefinitely in the background
    Ok(())
}
```

Finally, we can write another node that connects to the node that is hosting the `MyWorker` worker, and we are ready to send and receive messages between them.

```rust
use ockam_transport_websocket::{WebSocketTransport, WS};
use ockam_core::{route, Result};
use ockam_node::Context;
use ockam_macros::node;

#[ockam_macros::node(crate = "ockam_node")]
async fn main(mut ctx: Context) -> Result<()> {
    use ockam_node::MessageReceiveOptions;
let ws = WebSocketTransport::create(&ctx).await?;

    // Define the route to the server's worker.
    let r = route![(WS, "localhost:8000"), "my_worker"];

    // Now you can send messages to the worker.
    ctx.send(r, "Hello Ockam!".to_string()).await?;

    // Or receive messages from the server.
    let reply = ctx.receive::<String>().await?;

    // Stop all workers, stop the node, cleanup and return.
    ctx.stop().await
}
```


## Usage

Add this to your `Cargo.toml`:

```
[dependencies]
ockam_transport_websocket = "0.109.0"
```

## License

This code is licensed under the terms of the [Apache License 2.0][license-link].

[main-ockam-crate-link]: https://crates.io/crates/ockam

[crate-image]: https://img.shields.io/crates/v/ockam_transport_websocket.svg
[crate-link]: https://crates.io/crates/ockam_transport_websocket

[docs-image]: https://docs.rs/ockam_transport_websocket/badge.svg
[docs-link]: https://docs.rs/ockam_transport_websocket

[license-image]: https://img.shields.io/badge/License-Apache%202.0-green.svg
[license-link]: https://github.com/build-trust/ockam/blob/HEAD/LICENSE

[discuss-image]: https://img.shields.io/badge/Discuss-Github%20Discussions-ff70b4.svg
[discuss-link]: https://github.com/build-trust/ockam/discussions
