```
title: Secure Channel over many transport hops
```

# Secure Channel over many transport hops

## Responder node

Create a new file at:

```
touch examples/09-secure-channel-over-many-transport-hops-responder.rs
```

Add the following code to this file:

```rust
// examples/09-secure-channel-over-many-transport-hops-responder.rs
// This node starts a tcp listener, a secure channel listener, and an echoer worker.
// It then runs forever waiting for messages.

use ockam::{Context, Result, SecureChannel, TcpTransport, Vault};
use ockam_get_started::Echoer;

#[ockam::node]
async fn main(mut ctx: Context) -> Result<()> {
    // Initialize the TCP Transport.
    let tcp = TcpTransport::create(&ctx).await?;

    // Create a TCP listener and wait for incoming connections.
    tcp.listen("127.0.0.1:4000").await?;

    let vault = Vault::create(&ctx)?;

    // Create a secure channel listener at address "secure_channel_listener"
    SecureChannel::create_listener(&mut ctx, "secure_channel_listener", &vault).await?;

    // Create an echoer worker
    ctx.start_worker("echoer", Echoer).await?;

    // Don't call ctx.stop() here so this node runs forever.
    Ok(())
}

```

## Middle node

Create a new file at:

```
touch examples/09-secure-channel-over-many-transport-hops-middle.rs
```

Add the following code to this file:

```rust
// examples/09-secure-channel-over-many-transport-hops-middle.rs
// This node creates a tcp connection to a node at 127.0.0.1:4000
// Starts a tcp listener at 127.0.0.1:3000
// It then runs forever waiting to route messages.

use ockam::{Context, Result, TcpTransport};

#[ockam::node]
async fn main(ctx: Context) -> Result<()> {
    // Initialize the TCP Transport.
    let tcp = TcpTransport::create(&ctx).await?;

    // Create a TCP connection
    tcp.connect("127.0.0.1:4000").await?;

    // Create a TCP listener and wait for incoming connections.
    tcp.listen("127.0.0.1:3000").await?;

    // Don't call ctx.stop() here so this node runs forever.
    Ok(())
}

```

## Initiator node


Create a new file at:

```
touch examples/09-secure-channel-over-many-transport-hops-initiator.rs
```

Add the following code to this file:

```rust
// examples/09-secure-channel-over-many-transport-hops-initiator.rs
// This node creates an end-to-end encrypted secure channel over two tcp transport hops.
// It then routes a message, to a worker on a different node, through this encrypted channel.

use ockam::{Context, Result, Route, SecureChannel, TcpTransport, Vault, TCP};

#[ockam::node]
async fn main(mut ctx: Context) -> Result<()> {
    // Initialize the TCP Transport.
    let tcp = TcpTransport::create(&ctx).await?;

    // Create a TCP connection.
    tcp.connect("127.0.0.1:3000").await?;

    let vault = Vault::create(&ctx)?;

    // Connect to a secure channel listener and perform a handshake.
    let channel = SecureChannel::create(
        &mut ctx,
        // route to the secure channel listener
        Route::new()
            .append_t(TCP, "127.0.0.1:3000") // middle node
            .append_t(TCP, "127.0.0.1:4000") // responder node
            .append("secure_channel_listener"), // secure_channel_listener on responder node,
        &vault,
    )
        .await?;

    // Send a message to the echoer worker via the channel.
    ctx.send(
        Route::new().append(channel.address()).append("echoer"),
        "Hello Ockam!".to_string(),
    )
        .await?;

    // Wait to receive a reply and print it.
    let reply = ctx.receive::<String>().await?;
    println!("App Received: {}", reply); // should print "Hello Ockam!"

    // Stop all workers, stop the node, cleanup and return.
    ctx.stop().await
}

```

## Run

```
cargo run --example 09-secure-channel-over-many-transport-hops-responder
```

```
cargo run --example 09-secure-channel-over-many-transport-hops-middle
```

```
cargo run --example 09-secure-channel-over-many-transport-hops-initiator
```
