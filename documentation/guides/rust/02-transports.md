---
title: Transports
order: 3
---

# Transports

Ockam uses pluggable libraries to support multiple networking protocols. These are called transports. In this example we
will use the Ockam TCP Transport, available as the [ockam_transport_tcp](https://crates.io/crates/ockam_transport_tcp) crate.

# Client and Server

This example consists of two programs, a `client` and a `server`. The `server` listens for String messages on TCP port 10222.
When the `server` receives a message, it responds back to the `client` with the same message. The `client` connects to the `server`
and sends an initial message.

There are several ways you can configure your project to have two binaries. The easiest way, and the way that we will use
in this example is to use the `examples` directory.

1. Create an `examples` directory.
1. Create `client.rs` and `server.rs` source files in the `examples` directory. Later, place the appropriate code in these files.
1. The programs can be executed using cargo: `cargo run --example server` and `cargo run --example client`.

Add the Ockam dependencies to your project:

```toml
ockam = "0"
ockam_node = "0"
ockam_transport_tcp = "0"
```

## Server - Responder Worker

Now we must create a worker that performs an echo of a message. Since this worker doesn't need any state, we can use an
empty `struct`. Then the `Worker` trait is implemented (with the help of the `#[async_worker]` attribute).

The body of the worker's message handling function simply sends the message back.

```rust
use ockam::{async_worker, Context, Result, Routed, Worker};
use ockam_transport_tcp::TcpRouter;
use std::net::SocketAddr;

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

## Server - Listening for messages

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

## Server - Putting it all together

We can now implement the `main` function of the `server` and complete the program:

```rust
use ockam::{async_worker, Context, Result, Routed, Worker};
use ockam_transport_tcp::TcpRouter;
use std::net::SocketAddr;

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

# Client

Our `client` is much simpler, and much of the code is similar to the `server` code. This `client` code does not need to
implement a worker, it uses the APIs available on the Node's [Context](https://docs.rs/ockam_node/0.4.0/ockam_node/struct.Context.html).

## Client - Connecting to a Peer

Connecting to a remote peer involves calling a few functions in the [Ockam TCP Transport](https://docs.rs/ockam_transport_tcp/0.2.0/ockam_transport_tcp/index.html) crate.

1. First, we create and register the router itself by calling [register](https://docs.rs/ockam_transport_tcp/0.2.0/ockam_transport_tcp/struct.TcpRouter.html#method.register)
1. Next, we create a TCP worker specific to our new connection to the peer, by calling [start_tcp_worker](https://docs.rs/ockam_transport_tcp/0.2.0/ockam_transport_tcp/fn.start_tcp_worker.html)
1. Finally, we register the peer TCP worker with the router.

After the TCP worker for the peer has been registered, we can send and receive messages just like a local worker.

## Client - Routing Messages

Ockam Messages include their own hop-by-hop routing information. When sending a Message to a remote worker, we need to
specify the route it is going to take.

A Route is an ordered list of addresses. Each address has an address type, and the address value. Address types specify
the kind of transport the Address is associated with. Addresses which begin with a '0' are locally routed messages.
Addresses that begin with a '1' are TCP addresses. You will see these numbers in your Ockam addresses. Local workers
can choose to register explicit worker names, in which case the '0' local address type is not necessary.

For example, the Ockam Hub node runs a worker called `echo_service` that echoes any message that has been sent. There is
no need to address this service as '0.echo_service'.

For our client and server example, we need to construct a route that has the TCP peer information:

```rust
Route::new()
            .append(format!("1#{}", peer_addr))
            .append("echo_service")
```

The route built here will:
- First be sent to the peer address (over address type 1, a technical detail)
- Then be sent to the local address "echo_service" on the remote node.


## Client - Putting it all together

The main changes between Server and Client:
- A utility function to get the peer address.
- The `main` function starts a TCP Worker for an outbound connection.

```rust
use ockam::{Context, Result, Route};
use ockam_transport_tcp::{self as tcp, TcpRouter};
use std::net::SocketAddr;

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
