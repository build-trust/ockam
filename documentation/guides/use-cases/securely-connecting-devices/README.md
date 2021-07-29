# Securely connecting devices with Ockam Secure Channels

## Introduction

In the world of connected devices data often is transferred in plain text.

Most existing network security mechanism (like TLS) only secure communication between a device and the host it's connecting to, leaving the message exposed to all intermediate services.

A solution to that is End-to-end encryption.

Devices often cannot connect to each other directly because they are not exposed to the internet over a public hostname or IP address.

Devices have to rely on cloud services to forward communication and facilitate discovery.

Ockam provides Secure Channels tool to establish end-to-end encrypted communication across arbitrary number of network steps

## Setup

In this example we have two devices: Alice and Bob, which can't connect to each other directly.

They are going to use a cloud server called Hub Node to connect and forward messages between them.

Our goal is to make sure that Hub Node can facilitate communication, but cannot decrypt the messages Alice and Bob are exchanging.

## Server

A Hub Nodes are provided by the Ockam Hub cloud service.

You can either use the shared server at TCP address: `54.151.52.111:4000`

Or you can create your personal node by going to: https://hub.ockam.network


## Rust client application

### Project setup

1. Install Rust
    `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
1. Setup a hello_ockam Cargo Project to get started with Ockam
    `cargo new --lib hello_ockam && cd hello_ockam && mkdir examples && echo 'ockam = "*"' >> Cargo.toml && cargo build`

### Application code

The Alice node is going to establish a Secure Channel with the Bob node.

For that, the Bob node needs to register with the Hub Node first and get a Forwarding Address for Alice to access it.

Let's create Bob code:

```
touch examples/bob.rs
```


```rust
// examples/bob.rs

use ockam::{
    route, Context, Entity, RemoteForwarder, Result, SecureChannels, TcpTransport,
    TrustEveryonePolicy, Vault, TCP,
};

#[ockam::node]
async fn main(mut ctx: Context) -> Result<()> {
    // Using a shared Hub Node.
    // You can create a personal node by going to https://hub.ockam.network
    let hub_node_tcp_address = "54.151.52.111:4000";

    // Initialize the TCP Transport.
    let _tcp = TcpTransport::create(&ctx).await?;

    let vault = Vault::create(&ctx).expect("failed to create vault");
    let mut bob = Entity::create(&ctx, &vault)?;

    // Create a secure channel listener at address "bob_secure_channel_listener"
    bob.create_secure_channel_listener("bob_secure_channel_listener", TrustEveryonePolicy)?;

    let forwarder = RemoteForwarder::create(
        &ctx,
        route![(TCP, hub_node_tcp_address)],
        "bob_secure_channel_listener",
    )
    .await?;

    println!("Forwarding address: {}", forwarder.remote_address());

    let message = ctx.receive_timeout::<String>(10000).await?;
    println!("Bob Received: {} from Alice via secure channel", message); // should print "Hello Ockam!"

    ctx.stop().await
}
```

You should start the Bob in order to get the Forwarding Address:

```
cargo run --example bob
```

You would see some logs, including `Forwarding address: ...` - this address you should use when running Alice

Now create the Alice code:

```
touch examples/alice.rs
```

```rust
// examples/alice.rs

use ockam::{
    route, Context, Entity, Result, SecureChannels, TcpTransport, TrustEveryonePolicy, Vault, TCP,
};

#[ockam::node]
async fn main(ctx: Context) -> Result<()> {
    // Using a shared Hub Node.
    // You can create a personal node by going to https://hub.ockam.network
    let hub_node_tcp_address = "54.151.52.111:4000";

    let forwarding_address = "<Paste the forwarding address of Bob here>";

    // Initialize the TCP Transport.
    let _tcp = TcpTransport::create(&ctx).await?;

    let vault = Vault::create(&ctx).expect("failed to create vault");
    let mut alice = Entity::create(&ctx, &vault)?;

    let hub_node_route = route![(TCP, hub_node_tcp_address), forwarding_address];
    let channel = alice.create_secure_channel(hub_node_route, TrustEveryonePolicy)?;

    ctx.send(route![channel, "app"], "Hello Ockam!".to_string())
        .await?;

    Ok(())
}
```

and run the Alice code:

```
cargo run --example alice
```

You should see the log message: `App Received: Hello world`

## What is happening?

The example program is using Ockam framework to establish connection to a server in Ockam Hub.

Then using this connection, the two programs establish an [Ockam Secure Channel](../../rust/06-secure-channels)

Secure channel encrypts the messages with a key only available to devices and the server cannot decrypt them.

So we have the cloud service facilitating connection between devices, while not being able to decrypt the messages.

### Message flow

<img src="./Sequence.png" width="100%">


## What's next?

- More about the [Ockam framework](../../rust/)
- More about [Secure Channels](../../rust/06-secure-channels)
- More about [Ockam Hub](../../rust/07-hub)


