---
title: Forwarding
order: 6
---

# Have questions? Let us help!

**We are here to help.** See the [Guides And Demos](https://github.com/ockam-network/ockam/discussions/1134) in
GitHub Discussions.

# Forwarding

This example shows how you can route messages through remote nodes. With message forwarding, only the forwarding
address is needed to send messages to a worker. A forwarding address is an alias to a route. By using a forwarding address,
messages don't need to contain the entire route to a worker.

## Register with Ockam Hub Forwarding Service

Send a `register` message from your worker to the forwarding service running on Ockam Hub.

```rust
ctx.send_message(
    Route::new()
        .append_t(1, "Paste the address of the node you created on Ockam Hub here.")
        .append("forwarding_service"),
    "register".to_string(),
).await
```

## Get the forwarding address

Your worker will be notified with its forwarding address. This is done when the message body consists of the word `register`.
The forwarding address is given in the `reply` field. Messages sent to this address on the Hub will be forwarded to the
local worker.

When we receive a message indicating successful registration, the forwarding address of the `echo_service` is printed.

Copy the address printed after registration succeeds. The address is hexadecimal with a prefix of '0.'. You do not need
to copy the '0.' portion of the address.

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

## Send a message to the forwarding address

The `echo_client` that you built in the previous example can be used to send messages to the forwarding address of the
`echo_service` in the hub. The remote node address should be set to your hub address. After the hub entry in the route,
copy and paste the `echo_service` forwarding address.

```rust
use ockam::{Context, Result, Route};
use ockam_transport_tcp::{self as tcp, TcpRouter};
use std::net::SocketAddr;

#[ockam::node]
async fn main(mut ctx: Context) -> Result<()> {
    let remote_node = "Paste the address of the node you created on Ockam Hub here.";
    let echo_service = "Paste the forwarded address that the server received from registration here.";

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

# Putting it all Together - Forwarded Echo Service

```rust
use ockam::{async_worker, Context, Result, Route, Routed, Worker};
use ockam_transport_tcp::{self as tcp, TcpRouter};
use std::net::SocketAddr;

struct EchoService;

const HUB_ADDRESS: &str = "Paste the address of the node you created on Ockam Hub here.";

#[async_worker]
impl Worker for EchoService {
    type Message = String;
    type Context = Context;

    async fn initialize(&mut self, ctx: &mut Self::Context) -> Result<()> {
        // Send a "register" event to the Hub. The hub will reply with a forwarding address.
        ctx.send_message(
            Route::new()
                .append_t(1, HUB_ADDRESS)
                .append("forwarding_service"),
            "register".to_string(),
        )
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
    let router = TcpRouter::register(&ctx).await?;
    let hub_connection =
        tcp::start_tcp_worker(&ctx, HUB_ADDRESS.parse::<SocketAddr>().unwrap()).await?;

    router.register(&hub_connection).await?;

    ctx.start_worker("echo_service", EchoService).await
}

```

# Putting it all together - Echo Client

```rust
use ockam::{Context, Result, Route};
use ockam_transport_tcp::{self as tcp, TcpRouter};
use std::net::SocketAddr;

#[ockam::node]
async fn main(mut ctx: Context) -> Result<()> {
let remote_node = "Paste the address of the node you created on Ockam Hub here.";
let echo_service = "Paste the forwarded address that the server received from registration here.";

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
