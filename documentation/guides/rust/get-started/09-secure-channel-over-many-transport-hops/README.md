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

use ockam::{Context, Entity, Result, SecureChannels, TcpTransport, TrustEveryonePolicy, Vault};
use ockam_get_started::Echoer;

#[ockam::node]
async fn main(ctx: Context) -> Result<()> {
    ctx.start_worker("echoer", Echoer).await?;

    // Initialize the TCP Transport.
    let tcp = TcpTransport::create(&ctx).await?;

    // Create a TCP listener and wait for incoming connections.
    tcp.listen("127.0.0.1:4000").await?;

    let bob_vault = Vault::create(&ctx).expect("failed to create vault");
    let mut bob = Entity::create(&ctx, &bob_vault)?;

    // Create a secure channel listener at address "bob_secure_channel_listener"
    bob.create_secure_channel_listener("bob_secure_channel_listener", TrustEveryonePolicy)?;

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

use ockam::{
    route, Context, Entity, Result, SecureChannels, TcpTransport, TrustEveryonePolicy, Vault, TCP,
};

#[ockam::node]
async fn main(mut ctx: Context) -> Result<()> {
    // Initialize the TCP Transport.
    let _tcp = TcpTransport::create(&ctx).await?;

    let alice_vault = Vault::create(&ctx).expect("failed to create vault");
    let mut alice = Entity::create(&ctx, &alice_vault)?;
    let route = route![
        (TCP, "localhost:3000"),
        (TCP, "localhost:4000"),
        "bob_secure_channel_listener"
    ];

    // Connect to a secure channel listener and perform a handshake.
    let channel = alice.create_secure_channel(route, TrustEveryonePolicy)?;

    // Send a message to the echoer worker via the channel.
    let echoer_route = route![channel, "echoer"];

    ctx.send(echoer_route, "Hello Ockam!".to_string()).await?;

    // Wait to receive a reply and print it.
    let reply = ctx.receive::<String>().await?;
    println!("App Received: {}", reply); // should print "Hello Ockam!"
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

<div style="display: none; visibility: hidden;">
<hr><b>Next:</b> <a href="../10-secure-channel-with-entity">10. Secure Channel with Entity</a>
</div>
