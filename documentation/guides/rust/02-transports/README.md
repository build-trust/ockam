---
title: Transports
order: 4
---

#### Have questions? Let us help!

**We are here to help.** See the [Guides And Demos](https://github.com/ockam-network/ockam/discussions/1134) in
GitHub Discussions.

# Transports

Ockam uses pluggable libraries to support multiple networking protocols. These are called transports. In this example we
will use the Ockam TCP Transport, available as the [ockam_transport_tcp](https://crates.io/crates/ockam_transport_tcp) crate.

Add the Ockam TCP Transport dependency to your project:

```toml
ockam_transport_tcp = "0"
```

### Listen for messages over TCP

The `echo_service` only needs to call `TcpRouter::bind` to listen on a local port.

```rust
TcpRouter::bind(&ctx, "127.0.0.1:10222".parse::<SocketAddr>().unwrap()).await?;
```

# Putting it all together - Echo Service

```rust
use ockam::{async_worker, Context, Result, Routed, Worker};
use ockam_transport_tcp::TcpRouter;
use std::net::SocketAddr;

struct EchoService;

#[async_worker]
impl Worker for EchoService {
    type Message = String;
    type Context = Context;

    async fn handle_message(&mut self, ctx: &mut Context, msg: Routed<String>) -> Result<()> {
        println!("echo_service: {}", msg);
        ctx.send_message(msg.reply(), msg.take()).await
    }
}

#[ockam::node]
async fn main(ctx: Context) -> Result<()> {
    let _router = TcpRouter::bind(&ctx, "127.0.0.1:10222".parse::<SocketAddr>().unwrap()).await?;
    ctx.start_worker("echo_service", EchoService).await
}

```

Run the example:

```shell
cargo run --example echo_service
```

## Echo Client

The `echo_client` does not need to implement a worker, it uses the APIs available on the node's context.

### Connect to a remote node

We will connect to a remote node using the [ockam_transport_tcp](https://crates.io/crates/ockam_transport_tcp) crate.

1. First, we create and register a `TcpRouter` by calling `register`.
1. Next, we create a TCP worker specific to the remote node by calling `start_tcp_worker`.
1. Finally, we register this new TCP worker with the router.

After this registration process, we can send and receive messages just like a local worker.

### How messages are routed

Ockam Messages include their own hop-by-hop routing information. When a message is sent to a remote worker, the route it
is going to take must be specified.

A Route is an ordered list of addresses. Each address has an address type, and the address value. Address types specify
the kind of transport the Address is associated with. Addresses which begin with a '0' are locally routed messages.
Addresses that begin with a '1' are TCP addresses. You will see these numbers in your Ockam addresses. Local workers
can choose to register explicit worker names, in which case the '0' local address type is not necessary.

For example, the Ockam Hub node runs a worker called `echo_service` that echoes any message that it receives. There is
no need to address this service as '0.echo_service'.

For our `echo_client` and `echo_service` example, we need to construct a route that has the TCP remote node information:

```rust
Route::new().append_t(1, remote_node).append("echo_service"),
```

The route built above will be the path taken by the message.

**Hop 1**: The remote node over TCP.
**Hop 2**: The "echo_service" on the remote node.

# Putting it all together - Echo Client

```rust
use ockam::{Context, Result, Route};
use ockam_transport_tcp::{self as tcp, TcpRouter};
use std::net::SocketAddr;

#[ockam::node]
async fn main(mut ctx: Context) -> Result<()> {
    let remote_node = "127.0.0.1:10222";

    // Create and register a connection
    let router = TcpRouter::register(&ctx).await?;
    let connection =
        tcp::start_tcp_worker(&ctx, remote_node.parse::<SocketAddr>().unwrap()).await?;
    router.register(&connection).await?;

    ctx.send_message(
        Route::new().append_t(1, remote_node).append("echo_service"),
        "Hello Ockam!".to_string(),
    )
    .await?;

    // Then wait for a message back!
    let msg = ctx.receive::<String>().await?;
    println!("Received echo: '{}'", msg);
    ctx.stop().await
}

```

Make sure the `echo_server` is started first.

Run the example:

```shell
cargo run --example echo_client
```

The Ockam Hub [creates a node for you instantly](../03-hub). Let's move the `echo_service` to a node on the hub.
