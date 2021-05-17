```
title: Secure Channel
```

# Secure Channel

Now that we understand the basics of [Nodes](../01-node),
[Workers](../02-worker), and [Routing](../03-routing) ... let's
create our first encrypted secure channel.

Establishing a secure channel requires establishing a shared secret key between
the two entities that wish to communicate securely. This is usually achieved
using a cryptographic key agreement protocol to safely derive a shared secret
without transporting it over the network. In Ockam, we currently have support for
two different key agreement protocols - one based on the Noise Protocol Framework
and another based on Signal's X3DH design.

Running such protocols requires a stateful exchange of multiple messages and having
a worker and routing system allows Ockam to hide the complexity of creating and
maintaining a secure channel behind two simple functions:

* `SecureChannel::create_listener(...)` which waits for requests to create a secure channel.
* `SecureChannel::create(...)` which initiates the protocol to create a secure channel with a listener.

The `SecureChannel` APIs requires a Vault to store and manage secrets. The Vault is started like any other worker, and
we pass its address to the `SecureChannel` functions.

## App worker

For demonstration, we'll create a secure channel within a single node. Like our
previous example, [Workers](../02-worker), we'll create an `"echoer"` worker and
send it a message, but we'll route the message through a secure channel:

Create a new file at:

```
touch examples/05-secure-channel.rs
```

Add the following code to this file:

```rust
// examples/05-secure-channel.rs
// This node creates a secure channel and routes a message through it.

use ockam::{Context, Result, Route, SecureChannel, Vault};
use ockam_get_started::Echoer;

#[ockam::node]
async fn main(mut ctx: Context) -> Result<()> {
    // Start an Echoer worker at address "echoer"
    ctx.start_worker("echoer", Echoer).await?;

    let vault = Vault::create(&ctx)?;

    // Create a secure channel listener.
    SecureChannel::create_listener(&mut ctx, "secure_channel_listener", &vault).await?;

    // Connect to a secure channel listener and perform a handshake.
    let channel = SecureChannel::create(&mut ctx, "secure_channel_listener", &vault).await?;

    // Send a message to the echoer worker, via the secure channel.
    ctx.send(
        // route to the "echoer" worker via the secure channel.
        Route::new().append(channel.address()).append("echoer"),
        // the message you want echo-ed back
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

To run this new node program:

```
cargo run --example 05-secure-channel
```

Note how we send the messages on this route
`Route::new().append(channel.address()).append("echoer")`.
This encrypts the message when it enters the channel and decrypts it
when it exits.

## Message Flow

<img src="./sequence.png" width="100%">

<div style="display: none; visibility: hidden;">
<hr><b>Next:</b> <a href="../06-secure-channel-many-hops">06. Secure Channel over many hops</a>
</div>
