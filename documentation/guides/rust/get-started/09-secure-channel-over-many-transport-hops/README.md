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

use ockam::{Context, Result, SecureChannel};
use ockam_get_started::Echoer;
use ockam_transport_tcp::TcpTransport;

#[ockam::node]
async fn main(mut ctx: Context) -> Result<()> {
    let tcp = TcpTransport::create(&ctx).await?;
    tcp.listen("127.0.0.1:6000").await?;

    SecureChannel::create_listener(&mut ctx, "secure_channel_listener").await?;

    // Create an echoer worker
    ctx.start_worker("echoer", Echoer).await?;

    // This node never shuts down.
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

use ockam::{Context, Result};
use ockam_transport_tcp::TcpTransport;

#[ockam::node]
async fn main(ctx: Context) -> Result<()> {
    let tcp = TcpTransport::create(&ctx).await?;
    tcp.listen("127.0.0.1:4000").await?;
    tcp.connect("127.0.0.1:6000").await?;

    // This node never shuts down.
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

use ockam::{Context, Result, Route, SecureChannel};
use ockam_transport_tcp::{TcpTransport, TCP};

#[ockam::node]
async fn main(mut ctx: Context) -> Result<()> {
    let tcp = TcpTransport::create(&ctx).await?;
    tcp.connect("127.0.0.1:4000").await?;

    let route_to_listener =
        Route::new()
            .append_t(TCP, "127.0.0.1:4000") // middle node
            .append_t(TCP, "127.0.0.1:6000") // responder node
            .append("secure_channel_listener"); // secure_channel_listener on responder node
    let channel = SecureChannel::create(&mut ctx, route_to_listener).await?;

    // Send a message to the echoer worker via the channel.
    ctx.send(
        Route::new().append(channel.address()).append("echoer"),
        "Hello Ockam!".to_string(),
    )
    .await?;

    // Wait to receive a reply and print it.
    let reply = ctx.receive::<String>().await?;
    println!("Initiator Received: {}", reply); // should print "Hello Ockam!"

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
