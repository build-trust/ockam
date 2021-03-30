---
title: Hub
order: 5
---

# Hub

Ockam Hub is a service provided by Ockam that allows you to easily test Ockam Node based networks. Registering on
Hub creates an Ockam Node in the cloud, where you can relay messages between nodes.

Sending messages to a node on Ockam Hub is just like sending messages to a local worker, or a worker on another node:
a Route is constructed that describes the path to the destination worker.

## Getting started with Ockam Hub

Follow these steps to get started with Ockam Hub:

1. Sign in to Ockam Hub at [https://hub.ockam.network/](https://hub.ockam.network/) with your GitHub account.
1. After your node is deployed, you will see a confirmation page with instructions.
1. Find and save the address of the node. You will use this hostname in your code to build a route to the Hub.


## Sending an Echo to Hub

To send an echo message to your hub, just change the remote node address in the `echo_client` from Step 2.

```rust
use ockam::{Context, Result, Route};
use ockam_transport_tcp::{self as tcp, TcpRouter};
use std::net::SocketAddr;

#[ockam::node]
async fn main(mut ctx: Context) -> Result<()> {
    let remote_node = "<Ockam Hub address>"; // Ockam Hub

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
    println!("Received return message: '{}'", msg);
    ctx.stop().await
}

```

Sending messages to workers is even easier with <a href="04-forwarding">forwarding addresses</a>. The next step registers the
`echo_service` with the forwarding service. The forwarding service sends a forwarding address, that we can use to send
messages to the worker. The forwarding address is an alias for a route to a worker.
