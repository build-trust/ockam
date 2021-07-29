```yaml
title: Transports
```

# Transports

Ockam Transports are logical connections between Ockam Nodes. Ockam Transports
are an abstraction on top of physical transport protocols. The Ockam TCP
Transport is an implementation of an Ockam Transport using the TCP protocol.
This functionality is available in the `ockam_transport_tcp` crate, and is
included in the standard feature set of the top level `ockam` crate.

## Using the TCP Transport

The Ockam TCP Transport API fundamental type is `TcpTransport`. This type
provides the ability to create, connect, and listen for TCP connections. To
create a TCP transport, the Context is passed to the `create` function:

```rust
let tcp = TcpTransport::create(&ctx).await
```

The return value of `create` is a handle to the transport itself, which is used
for `connect` and `listen` calls. Listening on a local port is accomplished by
using the `listen` method. This method takes a string containing the IP address
and port, delimited by `:`. For example, this statement will listen on
localhost port 3000:

```rust
tcp.listen("127.0.0.1:3000").await
```

## Routing over Transports

Transports are implemented as workers, and have a unique address. The transport
address is used in routes to indicate that the message must be routed to the
remote peer.

Transport addresses also encode a unique protocol identifier. This identifier
is prefixed to the beginning of an address, followed by a `#`. The portion of
an address after the `#` is transport protocol specific. The TCP transport has
a transport protocol identifier of `1`, which is also aliased to the constant
`TCP`. The actual address uses the familiar `IP:PORT` format. A complete TCP
transport address could appear such as `1#127.0.0.1:3000`.

Transport addresses can be created using a tuple syntax to specify both
protocol id (TCP) and address:

```rust
// Implicit conversion from tuple to address
let route = route![(TCP, "10.0.0.1:8000")];
```

To send a message to a worker on another node connected by a transport, the
address of the transport is added to the route first, followed by the address
of the destination worker.

```rust
// This route forwards a message to the remote TCP peer Node
// and then to Worker "b"
let route = route![(TCP, "127.0.0.1:3000"), "b"]
```

Let's build an example that routes messages over many hops.

## Example: Responder

Create a new file at:

```
touch examples/04-routing-responder.rs
```

Add the following code to this file:

```rust
// examples/04-routing-responder.rs
// This node starts a tcp listener and an echoer worker.
// It then runs forever waiting for messages.

use ockam::{Context, Result, TcpTransport};
use hello_ockam::Echoer;

#[ockam::node]
async fn main(ctx: Context) -> Result<()> {
    // Initialize the TCP Transport.
    let tcp = TcpTransport::create(&ctx).await?;

    // Create a TCP listener and wait for incoming connections.
    tcp.listen("127.0.0.1:4000").await?;

    // Create an echoer worker
    ctx.start_worker("echoer", Echoer).await?;

    // Don't call ctx.stop() here so this node runs forever.
    Ok(())
}
```

## Example: Initiator

Create a new file at:

```
touch examples/04-routing-initiator.rs
```

Add the following code to this file:

```rust
// examples/04-routing-initiator.rs
// This node routes a message, to a worker on a different node, over the tcp transport.

use ockam::{Context, Result, route, TcpTransport, TCP};

#[ockam::node]
async fn main(mut ctx: Context) -> Result<()> {
    // Initialize the TCP Transport.
    TcpTransport::create(&ctx).await?;

    // Send a message to the "echoer" worker, on a different node, over a tcp transport.
    ctx.send(route![(TCP, "127.0.0.1:4000"), "echoer"], "Hello Ockam!".to_string()).await?;

    // Wait to receive a reply and print it.
    let reply = ctx.receive::<String>().await?;
    println!("App Received: {}", reply); // should print "Hello Ockam!"

    // Stop all workers, stop the node, cleanup and return.
    ctx.stop().await
}
```

## Run

```
cargo run --example 04-routing-responder
```

```
cargo run --example 04-routing-initiator
```

## Message Flow

<img src="./sequence.png" width="100%">