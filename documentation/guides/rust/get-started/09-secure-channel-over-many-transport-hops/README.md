```
title: Secure Channel over many transport hops
```

# Secure Channel over many transport hops

```rust
// examples/09-secure-channel-over-many-transport-hops-initiator.rs

use ockam::{Context, Result, Route, SecureChannel};
use ockam_transport_tcp::{TcpTransport, TCP};

#[ockam::node]
async fn main(mut ctx: Context) -> Result<()> {
    TcpTransport::create(&ctx, "127.0.0.1:4000").await?;


    let route_to_listener =
        Route::new()
            // Send a message to node B
            .append_t(TCP, "127.0.0.1:4000")
            // Send a message to node C
            .append_t(TCP, "127.0.0.1:6000")
            .append("secure_channel_listener");
    let channel = SecureChannel::create(&mut ctx, route_to_listener).await?;

    // Send a message to the echoer worker via the channel.
    ctx.send(
        Route::new()
            .append(channel.address())
            .append("echoer"),
        "Hello Ockam!".to_string()
    ).await?;

    // Wait to receive a reply and print it.
    let reply = ctx.receive::<String>().await?;
    println!("Initiator Received: {}", reply); // should print "Hello Ockam!"

    ctx.stop().await
}
```

```rust
// examples/09-secure-channel-over-many-transport-hops-middle.rs

use ockam::{Context, Result};
use ockam_transport_tcp::TcpTransport;

#[ockam::node]
async fn main(ctx: Context) -> Result<()> {
    TcpTransport::create_listener(&ctx, "127.0.0.1:4000").await?;
    TcpTransport::create(&ctx, "127.0.0.1:6000").await?;

    // This node never shuts down.
    Ok(())
}
```

```rust
// examples/09-secure-channel-over-many-transport-hops-responder.rs

use ockam::{Context, Result, SecureChannel};
use ockam_get_started::Echoer;
use ockam_transport_tcp::TcpTransport;

#[ockam::node]
async fn main(mut ctx: Context) -> Result<()> {
    TcpTransport::create_listener(&ctx, "127.0.0.1:6000").await?;

    SecureChannel::create_listener(&mut ctx, "secure_channel_listener").await?;

    // Create an echoer worker
    ctx.start_worker("echoer", Echoer).await?;

    // This node never shuts down.
    Ok(())
}
```
