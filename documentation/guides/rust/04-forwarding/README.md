---
title: Forwarding
order: 6
---

# Forwarding

This example shows how you can send messages to remote workers through the hub. With message forwarding, only the forwarding
address is needed to send messages to a worker. By using a forwarding address, workers do not need to know the details of
the network connection of other nodes and workers.

## Registering with Ockam Hub forwarding service

Send a registry message from your worker to the forwarding service running on Ockam Hub.

We can now use the `Worker::initialized` function in `echo_service` to perform this registration.

```rust
async fn initialize(&mut self, ctx: &mut Self::Context) -> Result<()> {
    let hub = "127.0.0.1:4000";

    let router = TcpRouter::register(&ctx).await?;
    let hub_connection =
        tcp::start_tcp_worker(&ctx, hub.parse::<SocketAddr>().unwrap()).await?;

    router.register(&hub_connection).await?;

    let forwarding_route = Route::new().append_t(1, hub).append("forwarding_service");

    // Send a "register" event to the Hub. The hub will reply with a forwarding address.
    ctx.send_message(forwarding_route, "register".to_string())
        .await
}
```

## Getting the forwarding address

Your worker will be notified with its forwarding address. This is done when the message body consists of the word `register`.
The forwarding address is given in the `reply` field. Messages sent to this address on the Hub will be forwarded to the
local worker.

When we receive a message indicating successful registration, the forwarding address of the `echo_service` is printed.

```rust
async fn handle_message(&mut self, ctx: &mut Context, msg: Routed<String>) -> Result<()> {
    if &msg.as_str() == &"register" {
        println!(
            "echo_service: My address on the hub is {}",
            msg.reply().recipient()
        );
        Ok(())
    } else {
        println!("echo_service: {}", msg);
        ctx.send_message(msg.reply(), msg.take()).await
    }
}
```

## Sending a message to the forwarding address

The `echo_client` from Step 3 can be re-used to send messages to the `echo_service` forwarding address in the hub. The remote
node address should be set to your hub address. After the hub entry in the route, copy and paste the `echo_service` forwarding address.

```rust
use ockam::{Context, Result, Route};
use ockam_transport_tcp::{self as tcp, TcpRouter};
use std::net::SocketAddr;

#[ockam::node]
async fn main(mut ctx: Context) -> Result<()> {
    let remote_node = "<Ockam Hub address>";
    let echo_service = "<hex address>"; // Replace with hub forwarding address

    // Create and register a connection
    let router = TcpRouter::register(&ctx).await?;
    let connection =
        tcp::start_tcp_worker(&ctx, remote_node.parse::<SocketAddr>().unwrap()).await?;
    router.register(&connection).await?;

    ctx.send_message(
        Route::new().append_t(1, remote_node).append(echo_service),
        "Hello Ockam!".to_string(),
    )
    .await?;

    // Then wait for a message back!
    let msg = ctx.receive::<String>().await?;
    println!("Received echo: '{}'", msg);
    ctx.stop().await
}

```

## Putting it all Together - Forwarded Echo Service

```rust
use ockam::{async_worker, Context, Result, Route, Routed, Worker};
use ockam_transport_tcp::{self as tcp, TcpRouter};
use std::net::SocketAddr;

struct EchoService;

#[async_worker]
impl Worker for EchoService {
type Message = String;
type Context = Context;

    async fn initialize(&mut self, ctx: &mut Self::Context) -> Result<()> {
        let hub = "127.0.0.1:4000";

        let router = TcpRouter::register(&ctx).await?;
        let hub_connection =
            tcp::start_tcp_worker(&ctx, hub.parse::<SocketAddr>().unwrap()).await?;

        router.register(&hub_connection).await?;

        let forwarding_route = Route::new().append_t(1, hub).append("forwarding_service");

        // Send a "register" event to the Hub. The hub will reply with a forwarding address.
        ctx.send_message(forwarding_route, "register".to_string())
            .await
    }

    async fn handle_message(&mut self, ctx: &mut Context, msg: Routed<String>) -> Result<()> {
        if &msg.as_str() == &"register" {
            println!(
                "echo_service: My address on the hub is {}",
                msg.reply().recipient()
            );
            Ok(())
        } else {
            println!("echo_service: {}", msg);
            ctx.send_message(msg.reply(), msg.take()).await
        }
    }
}

#[ockam::node]
async fn main(ctx: Context) -> Result<()> {
ctx.start_worker("echo_service", EchoService).await
}
```

## Putting it all together - Echo Client

```rust
use ockam::{Context, Result, Route};
use ockam_transport_tcp::{self as tcp, TcpRouter};
use std::net::SocketAddr;

#[ockam::node]
async fn main(mut ctx: Context) -> Result<()> {
let remote_node = "127.0.0.1:4000"; // Ockam Hub
let echo_service = "8f0e82ca"; // Replace with hub forwarding address

    // Create and register a connection
    let router = TcpRouter::register(&ctx).await?;
    let connection =
        tcp::start_tcp_worker(&ctx, remote_node.parse::<SocketAddr>().unwrap()).await?;
    router.register(&connection).await?;

    ctx.send_message(
        Route::new().append_t(1, remote_node).append(echo_service),
        "Hello Ockam!".to_string(),
    )
    .await?;

    // Then wait for a message back!
    let msg = ctx.receive::<String>().await?;
    println!("Received echo: '{}'", msg);
    ctx.stop().await
}
```
