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

We can now use the `Worker::initialzied` function in `echo_service` to perform this registration.

```rust
async fn initialize(&mut self, ctx: &mut Self::Context) -> Result<()> {
    let forwarding_route = Route::new()
        .append_t(1, self.hub_address.clone())
        .append("forwarding_service");

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
