---
title: Transports
order: 4
---

# Transports

Ockam uses pluggable libraries to support multiple networking protocols. These are called transports. In this example we
will use the Ockam TCP Transport, available as the [ockam_transport_tcp](https://crates.io/crates/ockam_transport_tcp) crate.

Add the Ockam TCP Transport dependency to your project:

```toml
ockam_transport_tcp = "0"
```

# Listening for messages over TCP

To extend the `echo_service` to listen on a TCP port, we add one API call to `TcpRouter::bind`.

```rust
TcpRouter::bind(&ctx, "127.0.0.1:10222".parse::<SocketAddr>().unwrap()).await?;
```

A good place to put setup code like this is in the `initialized` callback of the `Worker` trait. This code will run after
the node creates your worker.

The `echo_service` initialization executes the bind call:
```rust
  async fn initialize(&mut self, ctx: &mut Self::Context) -> Result<()> {
      let _router =
          TcpRouter::bind(&ctx, "127.0.0.1:10222".parse::<SocketAddr>().unwrap()).await?;
      Ok(())
  }
```

## Putting it all together - echo_service

```rust
use ockam::{async_worker, Context, Result, Routed, Worker};
use ockam_transport_tcp::TcpRouter;
use std::net::SocketAddr;

struct EchoService;

#[async_worker]
impl Worker for EchoService {
    type Message = String;
    type Context = Context;

    async fn initialize(&mut self, ctx: &mut Self::Context) -> Result<()> {
        let _router =
            TcpRouter::bind(&ctx, "127.0.0.1:10222".parse::<SocketAddr>().unwrap()).await?;
        Ok(())
    }

    async fn handle_message(&mut self, ctx: &mut Context, msg: Routed<String>) -> Result<()> {
        println!("echo_service: {}", msg);
        ctx.send_message(msg.reply(), msg.take()).await
    }
}

#[ockam::node]
async fn main(ctx: Context) -> Result<()> {
    ctx.start_worker("echo_service", EchoService).await
}

```

# echo_client

The `echo_client` does not need to implement a worker, it uses the APIs available on the node's context.

## Connecting to a Remote Node

Connecting to a remote peer involves calling a few functions in the [ockam_transport_tcp](https://crates.io/crates/ockam_transport_tcp) crate.

1. First, we create and register the router itself by calling `register`
1. Next, we create a TCP worker specific to the remote node by calling `start_tcp_worker`
1. Finally, we register this new TCP worker with the router.

After this registration process, we can send and receive messages just like a local worker.

## Routing Messages

Ockam Messages include their own hop-by-hop routing information. When sending a Message to a remote worker, we need to
specify the route it is going to take.

A Route is an ordered list of addresses. Each address has an address type, and the address value. Address types specify
the kind of transport the Address is associated with. Addresses which begin with a '0' are locally routed messages.
Addresses that begin with a '1' are TCP addresses. You will see these numbers in your Ockam addresses. Local workers
can choose to register explicit worker names, in which case the '0' local address type is not necessary.

For example, the Ockam Hub node runs a worker called `echo_service` that echoes any message that has been sent. There is
no need to address this service as '0.echo_service'.

For our `echo_client` and `echo_service` example, we need to construct a route that has the TCP remote node information:

```rust
Route::new().append_t(1, remote_node).append("echo_service"),
```

The route built here will be sent to:
1. The remote node address over TCP.
1. The local address "echo_service" on the remote node.

## Putting it all together - echo_client

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

The Ockam Hub <a href="03-hub">creates a node for you instantly</a> Let's move the `echo_service` to a node on the hub.
