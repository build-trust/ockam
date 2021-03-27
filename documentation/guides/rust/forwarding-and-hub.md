---
title: Message forwarding with Ockam Hub
order: 4
---

# Worker message forwarding  with Ockam Hub

This example introduces the concepts of node transports, and sending messages to remote workers.

## Getting started

Create two new Rust binaries with cargo:

```shell
cargo new msg_receiver
cargo new msg_sender
```

Add the `ockam`, `ockam_node`, and `ockam_transport_tcp` dependencies to both projects:

```toml
ockam = "0"
ockam_node = "0"
ockam_transport_tcp = "0"
```

Initialize your node as shown in the [Nodes and Workers]() example.

```rust
#[ockam::node]
async fn main(ctx: Context) -> Result<()> {
    ctx.stop().await
}
```

## Message receiver

Create and start a worker in the `msg_receiver` program as shown in the [Nodes and Workers]() example.
This worker registers itself with Ockam Hub to receive forwarded messages.

```rust
use ockam::{async_worker, Context, Result, Route, Routed, Worker};
use std::net::SocketAddr;

struct MsgReceive {
    hub: SocketAddr,
}

#[async_worker]
impl Worker for MsgReceive {
    type Context = Context;
    type Message = String;

    async fn initialize(&mut self, ctx: &mut Context) -> Result<()> {
        Ok(())
    }
}
```

## Registering with Ockam Hub forwarding service

Send a registry message from your worker to the forwarding service running on Ockam Hub.

```rust
let route = Route::new().append_t(1, self.peer).append("forwarding_service");
ctx.send_message(route, "register".to_string()).await?;
```

## Receiving the forwarding address

Your worker will be notified with its forwarding address.  Messages sent to that address on the Hub will be forwarded to it.

Note: this code will look slighty nicer when we merge some changes that are currently on the sanjo/kex_rebase branch!

```rust
async fn handle_message(&mut self, ctx: &mut Context, msg: Routed<String>) -> Result<()> {
    if msg.as_str() == "register" {
        info!("My forwarding address: {}", msg.reply());
    }
}
```

## Echo forwarded messages

```rust
async fn handle_message(&mut self, ctx: &mut Context, msg: Routed<String>) -> Result<()> {
    if msg.as_str() == "register" {
        info!("My forwarding address: {}", msg.reply());
    } else {
        let route = msg.reply();
        info!("Echoing message: {}", msg);
        ctx.send_message(route, msg.take()).await?;
    }
}
```


## Message sender

The sending client is much simpler.  Connect it to Ockam Hub as outlined in the Tcp guide.


```rust
async fn main(mut ctx: Context) -> Result<()> {
    // Get our peer address
    let peer = get_peer_addr();

    // Create and register a TcpRouter
    let rh = TcpRouter::register(&ctx).await?;

    // Create and register a connection worker pair
    let w_pair = tcp::start_tcp_worker(&ctx, peer.clone()).await?;
    rh.register(&w_pair).await?;

    Ok(())
}
```

Provide the forwarding address to the process.

```rust
let mut buffer = String::new();
println!("Paste the forwarding route below ↓");
io::stdin().read_line(&mut buffer).unwrap();
let route = Route::parse(buffer).unwrap_or_else(|| {
    error!("Failed to parse route!");
    eprintln!("Route format [type#]<address> [=> [type#]<address>]+");
    std::process::exit(1);
});
```

## Sending a message

You can now send a message to the forwarding service.

```rust
ctx.send_message(route, "This message goes via Hub".to_string()).await?;
```


## Wait for the reply

```rust
let echo = ctx.receive::<String>().await?;
info!("Echo message: {}", echo);
```


## Running the example

First run your message receiver.

```shell
cargo run --bin msg_receiver
...
My forwarding address: 1#127.0.0.1:4000 => 0#1a840b9c
```

In another shell, run the message sender, while providing the forwarding route.

```shell
echo "1#127.0.0.1:4000 => 0#1a840b9c" | cargo run --bin msg_sender
```
