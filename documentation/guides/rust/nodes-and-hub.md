---
title: Node networking with Ockam Hub
order: 3
---

# Worker networking with Ockam Hub

This example introduces the concepts of node transports, and sending messages to remote workers.

## Getting started with Ockam Hub

Ockam Hub is a service provided by Ockam that allows you to easily test Ockam Node based networks.

Follow these steps to get started with Ockam Hub:

1. Sign in to Ockam Hub at (https://hub.ockam.network/)[https://hub.ockam.network/] with your GitHub account.
1. After your Ockam Node is deployed, you will see a confirmation page with instructions.
  1. Find and save the hostname of the node. You will use this hostname in your code to build a route to the Hub.
  1. Alternatively, you can download a pre-configured code snippet by clicking the "**Get a pre-configured echo service example**" button.

We recommend using the hostname associated with the Hub instance, instead of the IP address. Most Ockam APIs take `SocketAddr`
arguments for remote hosts. You can turn your Ockam Hub hostname into a `SocketAddr` with the following code:

```rust
fn get_hub_address(host_and_port: &str) -> SocketAddr {
    if let Ok(addrs) = host_and_port.to_socket_addrs() {
        if addrs.len() != 0 {
            return addrs.as_slice()[0];
        }
    }
    panic!("Unable to resolve Ockam Hub address :(");
}
```

## Getting started with the Rust SDK

Now that you have a node running on Ockam Hub, let's send it a message!

Create a new Rust binary with cargo:

```shell
cargo new tcp_worker
```

Ockam's functionality is decoupled into several crates. For this example, we will need the crates for:

1. The base Ockam API (`ockam` crate)
1. The Ockam Node API, for workers and messaging (`ockam_node` crate)
1. The Ockam TCP Transport mechanism (`ockam_transport_tcp` crate). This is described in more detail below.

Add these three dependencies to your project:

```toml
ockam = "0"
ockam_node = "0"
ockam_transport_tcp = "0"
```

Initialize your node as shown in the [Nodes and Workers](/learn/how-to-guides/rust-sdk-code-examples/nodes-and-workers) example.

```rust
#[ockam::node]
async fn main(ctx: Context) -> Result<()> {
    ctx.stop().await
}
```

## Ockam Transports

Ockam uses pluggable transport libraries to interface with different networking protocols. The `ockam_tcp_transport`
crate provides workers that can listen, connect, and route messages over TCP. With it, you can connect to any other TCP transport
implementation.

First, import the required items from the TCP transport crate:

```rust
use ockam_transport_tcp::{self as tcp, TcpRouter};
use std::net::SocketAddr;
```

## Creating a TCP transport

Create and register the TCP domain router with your local node:

```rust
let router_handle = TcpRouter::register(&ctx).await?;
```

The TCP router ensures that messages sent over TCP are handled by the correct connection workers.

Now we need to create the TCP worker that establishes a connection to a remote node. This is where your Ockam Hub address
is used:

```rust
let addr: SocketAddr = get_hub_address("your.ockam.network:4000");
let connection = tcp::start_tcp_worker(&ctx, addr).await?;
```

Finally, we need to register the TCP worker with the router. This is done by using the `register` API.

```rust
router_handle.register(&connection).await?;
```

## Building Message Routes

Ockam Messages include their own hop-by-hop routing information. When sending a Message to a remote worker, we need to
specify the route it is going to take.

A Route is an ordered list of addresses. Each address has an address type, and the address value. Address types specify
the kind of transport the Address is associated with. Addresses which begin with a '0' are locally routed messages.
Addresses that begin with a '1' are TCP addresses. You will see these numbers in your Ockam addresses. Local workers
can choose to register explicit worker names, in which case the '0' local address type is not necessary.

For example, the Ockam Hub node runs a worker called `echo_service` that echoes any message that has been sent. There is
no need to address this service as '0.echo_service'.

The `Route` API is used to help build message routes. Let's build a route to or Ockam Hub `echo_service`:

```rust
// ... more imports
use ockam::Route;

let route = Route::new()
    .append_t(1, get_hub_address("your.ockam.network:4000"))
    .append("echo_service");
```

## Sending and Receiving an Echo Message

To send a message to `echo_service` and wait for the reply, we use the same APIs as for local workers:

```rust
ctx.send_message(route, "Hello you over there!".to_string()).await?;

let reply = ctx.receive::<String>().await?;
println!("Echo says: {}", reply);
```

## Running the example

From the `ockam_examples` crate, run:

```shell
cargo run --example guide_02_tcp_remote
```
