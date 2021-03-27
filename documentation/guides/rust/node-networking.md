---
title: Node Networking
order: 7
---

# Networking between Ockam Nodes

Ockam uses pluggable libraries to support multiple networking protocols. These are called transports. In this example we
will use the Ockam TCP Transport, available as the [ockam_transport_tcp](https://crates.io/crates/ockam_transport_tcp) crate.

## Client and Server

This example consists of two programs, a `client` and a `server`. The `server` listens for String messages on TCP port 10222.
When the `server` receives a message, it responds back to the `client` with the same message. The `client` connects to the `server`
and sends an initial message.

## Getting Started

Create a new Rust binary with cargo:

```shell
cargo new echo
```

There are several ways you can configure your project to have two binaries. The easiest way, and the way that we will use
in this example is to use the `examples` directory.

1. Create an `examples` directory.
1. Create `client.rs` and `server.rs` source files in the `examples` directory. Later, place the appropriate code in these files.
1. The programs can be executed using cargo: `cargo run --example server` and `cargo run --example client`.

Ockam's functionality is decoupled into several crates. For this example, we will need the crates for:

1. The base Ockam API ([ockam](https://crates.io/crates/ockam) crate)
1. The Ockam Node API, for workers and messaging ([ockam_node](https://crates.io/crates/ockam_node) crate)
1. The Ockam TCP Transport mechanism ([ockam_transport_tcp](https://crates.io/crates/ockam_transport_tcp) crate).

Add these three dependencies to your project:

```toml
ockam = "0"
ockam_node = "0"
ockam_transport_tcp = "0"
```

## Ockam Nodes and Workers

Both `client` and `server` are Ockam Nodes running Workers. In both source files, bootstrap the node:

```rust
#[ockam::node]
async fn main(ctx: Context) -> Result<()> {
    ctx.stop().await
}
```

The Ockam TCP transport has two components: a listener, and a connector. For this example, the `server` utilizes the TCP
listener and Router. The TCP Router is responsible for sending messages to the appropriate TCP worker. The `client` utilizes
the connector.

## Server

The `server` begins with a prelude of `use` statements, bringing in the Ockam Worker and Transport APIs:

```rust
use ockam::{async_worker, Context, Result, Routed, Worker};
use ockam_transport_tcp::TcpRouter;
use std::net::SocketAddr;
```

### Responder Worker

Now we must create a worker that performs an echo of a message. Since this worker doesn't need any state, we can use an
empty `struct`. Then the `Worker` trait is implemented (with the help of the `#[async_worker]` attribute).

The body of the worker's message handling function simply sends the message back.

```rust
struct Responder;

#[async_worker]
impl Worker for Responder {
    type Message = String;
    type Context = Context;

    async fn handle_message(&mut self, ctx: &mut Context, msg: Routed<String>) -> Result<()> {
        info!("Responder: {}", msg);
        ctx.send_message(msg.reply(), msg.take()).await?;
        Ok(())
    }
}
```

### Listening for messages

To listen for messages in the `server`, we need to create a `TCPRouter` and bind it to a TCP port. For example purposes,
this code can be used to listen on port `10222`.

```rust
fn get_bind_addr() -> SocketAddr {
    std::env::args()
        .skip(1)
        .take(1)
        .next()
        .unwrap_or(format!("127.0.0.1:10222"))
        .parse()
        .ok()
        .unwrap_or_else(|| {
            error!("Failed to parse socket address!");
            eprintln!("Usage: network_echo_server <ip>:<port>");
            std::process::exit(1);
        })
}
```

### Starting the `TCPRouter`

We can now implement the `main` function of the `server`:

```rust
#[ockam::node]
async fn main(mut ctx: Context) -> Result<()> {
    // Get either the default socket address, or a user-input
    let bind_addr = get_bind_addr();
    debug!("Binding to: {}", bind_addr);

    // Create a new _binding_ TcpRouter
    let _r = TcpRouter::bind(&ctx, bind_addr).await?;

    // Create the Responder worker
    ctx.start_worker("echo_service", Responder).await?;

    // The server never shuts down
    Ok(())
}
```

## Client

Our `client` is much simpler, and much of the code is similar to the `server` code. This `client` code does not need to
implement a worker, it uses the APIs available on the Node's [Context](https://docs.rs/ockam_node/0.4.0/ockam_node/struct.Context.html).

As with all programs, we import our dependencies:

```rust
use ockam::{Context, Result, Route};
use ockam_transport_tcp::{self as tcp, TcpRouter};
use std::net::SocketAddr;
```

Similar to the utility function which binds to a port, below is a function you can use to a `server` host and port:

```rust
fn get_peer_addr() -> SocketAddr {
    std::env::args()
        .skip(1)
        .take(1)
        .next()
        .unwrap_or(format!("127.0.0.1:10222"))
        .parse()
        .ok()
        .unwrap_or_else(|| {
            error!("Failed to parse socket address!");
            eprintln!("Usage: client <ip>:<port>");
            std::process::exit(1);
        })
}
```

The `main` function of `client` looks similar to `server`, with a few different APIs:

```rust
#[ockam::node]
async fn main(mut ctx: Context) -> Result<()> {
    // Get our peer address
    let peer_addr = get_peer_addr();

    // Create and register a TcpRouter
    let rh = TcpRouter::register(&ctx).await?;

    // Create and register a connection worker pair
    let conn = tcp::start_tcp_worker(&ctx, peer_addr).await?;
    rh.register(&conn).await?;

    // Send a message to the remote
    ctx.send_message(
        Route::new()
            .append(format!("1#{}", peer_addr))
            .append("echo_service"),
        String::from("Hello you over there!"),
    )
    .await?;

    // Then wait for a message back!
    let msg = ctx.receive::<String>().await?;
    info!("Received return message: '{}'", msg);

    ctx.stop().await?;
    Ok(())
}
```

## Running the example

From the `ockam_examples` crate, run:

WIP
