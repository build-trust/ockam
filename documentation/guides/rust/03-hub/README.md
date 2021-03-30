---
title: Hub
order: 5
---

# Have questions? Let us help!

**We are here to help.** See the [Guides And Demos](https://github.com/ockam-network/ockam/discussions/1134) in
GitHub Discussions.

# Hub

Ockam Hub is a service provided by Ockam that allows you to easily develop networks of Ockam nodes. When you register with The Hub, it creates an Ockam node in the cloud, where you can route messages between applications.

A worker sends a message to The Hub in the same way it sends messages to any other remote node. A route is constructed that describes the path to the destination worker.

## Getting started with Ockam Hub

Follow these steps to get started with Ockam Hub:

1. Sign in to Ockam Hub at [https://hub.ockam.network/](https://hub.ockam.network/) with your GitHub account.
1. After your node is deployed, you will see a confirmation page with the address of the node created for you.
1. Save the address of your node. You will use this address in the code below to build a route to the Hub.

## Sending an Echo to Hub

To send an echo message to your node, change the remote node address in the `echo_client` from Step 2.

```rust
use ockam::{Context, Result, Route};
use ockam_transport_tcp::{self as tcp, TcpRouter};
use std::net::SocketAddr;

#[ockam::node]
async fn main(mut ctx: Context) -> Result<()> {
    let remote_node = "Paste the address of the node you created on Ockam Hub here.";

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

It's even easier to send messages to remote nodes using a [forwarding addresses](../04-forwarding). The next step registers the
`echo_service` with the forwarding service. The forwarding service sends a forwarding address that we can use to send
messages to the worker. The forwarding address is an alias for a route to a worker on a remote node.
