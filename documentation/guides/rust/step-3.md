---
title: Node and Ockam Hub
order: 4
---

# Node and Ockam Hub

Ockam Hub is a service provided by Ockam that allows you to easily test Ockam Node based networks. Registering on
Hub creates an Ockam Node in the cloud, where you can relay messages between nodes.

Sending messages to a node on Ockam Hub is just like sending messages to a local worker, or a worker on another node:
a Route is constructed that describes the path to the destination worker.

## Getting started with Ockam Hub

Follow these steps to get started with Ockam Hub:

1. Sign in to Ockam Hub at (https://hub.ockam.network/)[https://hub.ockam.network/] with your GitHub account.
1. After your Ockam Node is deployed, you will see a confirmation page with instructions.
1. Find and save the hostname of the node. You will use this hostname in your code to build a route to the Hub.

We recommend using the hostname associated with the Hub instance, instead of the IP address. Most Ockam APIs take `SocketAddr`
arguments for remote hosts.

## Hub messages over TCP

Ensure your project has these dependencies, which will provide the Ockam TCP Transport.

```toml
ockam = "0"
ockam_node = "0"
ockam_transport_tcp = "0"
```

The steps to connect to the Hub node are the same as connecting to any peer using TCP:

1. Create and register a `TcpRouter` with your local node:
1. Create a TCP worker for the hub connection. This is where your Ockam Hub address is used.
1. Register the TCP worker with the `TcpRouter` using the `register` API.
1. Send messages using a route to the remote worker.
1. Receive messages using standard Worker APIs.

You can turn your Ockam Hub hostname into a `SocketAddr` by using the provided `get_hub_address` function in the example code.

## Sending and Receiving an Echo Message

The `Route` API is used to help build message routes. Let's build a route to or Ockam Hub `echo_service`:

```rust
// ... more imports
use ockam::Route;

let route = Route::new()
    .append_t(1, get_hub_address("your.ockam.network:4000"))
    .append("echo_service");
```

To send a message to `echo_service` and wait for the reply, we use the same APIs as for local workers:

```rust
ctx.send_message(route, "Hello you over there!".to_string()).await?;

let reply = ctx.receive::<String>().await?;
println!("Echo says: {}", reply);
```

## Putting it all together

```rust

use ockam_transport_tcp::{self as tcp, TcpRouter};
use std::net::SocketAddr;
use ockam::Route;

fn get_hub_address(host_and_port: &str) -> SocketAddr {
    if let Ok(addrs) = host_and_port.to_socket_addrs() {
        if addrs.len() != 0 {
            return addrs.as_slice()[0];
        }
    }
    panic!("Unable to resolve Ockam Hub address :(");
}

#[ockam::node]
async fn main(ctx: Context) -> Result<()> {
    let router_handle = TcpRouter::register(&ctx).await?;

    // Paste your Ockam Hub address here!
    let addr: SocketAddr = get_hub_address("your.ockam.network:4000");

    let connection = tcp::start_tcp_worker(&ctx, addr).await?;
    router_handle.register(&connection).await?;

    let route = Route::new()
      .append_t(1, get_hub_address("your.ockam.network:4000"))
      .append("echo_service");

    ctx.stop().await
}

```

