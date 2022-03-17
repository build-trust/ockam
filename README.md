<div align="center">
:sparkles:
<a href="./documentation/use-cases/end-to-end-encryption-with-rust#readme">
Hands-on Introduction: Build end-to-end encrypted, mutually-authenticated, secure messaging in Rust
</a>
:sparkles:
</div>

<hr>

<p>
<a href="./implementations#readme">
<img alt="Ockam" src="./documentation/concepts/assets/banner.png" width="900">
</a>
</p>

Rust and Elixir libraries for end-to-end encrypted, mutually authenticated, secure communication.

Data, within modern distributed applications, are rarely exchanged over a single point-to-point
transport connection. Application messages routinely flow over complex, multi-hop, multi-protocol
routes — _across data centers, through queues and caches, via gateways and brokers_ — before reaching
their end destination.

Transport layer security protocols are unable to protect application messages because their protection
is constrained by the length and duration of the underlying transport connection.

Ockam is a suite of programming libraries and infrastructure that makes it simple for our applications
to guarantee end-to-end integrity, authenticity, and confidentiality of data.

We no longer have to implicitly depend on the defenses of every machine or application within the same,
usually porous, network boundary. Our application's messages don't have to be vulnerable at every point,
along their journey, where a transport connection terminates.

Instead, our application can have a strikingly smaller vulnerability surface and easily make
_granular authorization decisions about all incoming information and commands._

<hr>

## Features

* End-to-end encrypted, mutually authenticated _secure channels_.
* Key establishment, rotation, and revocation - _for fleets, at scale_.
* Attribute-based Access Control - credentials with _selective disclosure_.
* Add-ons for a variety of operating environments, transport protocols, and _cryptographic hardware_.
* Libraries for multiple languages - _Rust, Elixir_ (more on the roadmap).

<hr>

## Hello Ockam

Let's write a simple example to create an encrypted secure channel between Alice and Bob. When a
message is sent through this channel it will be encrypted when it enters the channel and decrypted
just before it exits the channel.

For the purpose of our first example, we'll create a local channel within one program. In
[later examples](./documentation/guides/rust#readme), you'll see that it's just as easy to create
end-to-end protected channels over multi-hop, multi-protocol transport routes:

1. Install Rust

    ```
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
    ```

2. Setup a new cargo project to get started.

    ```
    cargo new --lib hello_ockam && cd hello_ockam && mkdir examples &&
      echo 'ockam = "*"' >> Cargo.toml && cargo build
    ```

    If the above instructions don't work on your machine, please
    [post a question](https://github.com/ockam-network/ockam/discussions/1642),
    we would love to help.

3. Create a file at `examples/hello.rs` and copy the below code snippet to it.

```rust
// examples/hello.rs
use ockam::{
    identity::{Identity, TrustEveryonePolicy},
    route,
    vault::Vault,
    Context, Result,
};

#[ockam::node]
async fn main(mut ctx: Context) -> Result<()> {
    // Create a Vault to safely store secret keys for Alice and Bob.
    let vault = Vault::create();

    // Create an Identity to represent Bob.
    let bob = Identity::create(&ctx, &vault).await?;

    // Create a secure channel listener for Bob that will wait for requests to
    // initiate an Authenticated Key Exchange.
    bob.create_secure_channel_listener("bob", TrustEveryonePolicy).await?;

    // Create an entity to represent Alice.
    let alice = Identity::create(&ctx, &vault).await?;

    // As Alice, connect to Bob's secure channel listener and perform an
    // Authenticated Key Exchange to establish an encrypted secure channel with Bob.
    let channel = alice.create_secure_channel("bob", TrustEveryonePolicy).await?;

    // Send a message, ** THROUGH ** the secure channel,
    // to the "app" worker on the other side.
    //
    // This message will automatically get encrypted when it enters the channel
    // and decrypted just before it exits the channel.
    ctx.send(route![channel, "app"], "Hello Ockam!".to_string()).await?;

    // Wait to receive a message for the "app" worker and print it.
    let message = ctx.receive::<String>().await?;
    println!("App Received: {}", message); // should print "Hello Ockam!"

    // Stop all workers, stop the node, cleanup and return.
    ctx.stop().await
}

```

4. Run the example

    ```
    cargo run --example hello
    ```

Congratulations on running your first Ockam program 🥳.

A lot happened when you ran this small example. It created a secure vault, spawned workers to represent
entities, established a mutually authenticated channel and then routed a message through that channel.
This involved running cryptographic protocols for generating keys, authenticating as an entity, performing
an authenticated key exchange and exchanging messages with authenticated encryption.

To learn more about how we make these powerful cryptographic protocols simple to use, please have a look
at our [step-by-step guide](./documentation/guides/rust#readme) where we introduce the building blocks in Ockam.

<hr>

## Concepts

Here's a high-level view of the core ideas in Ockam.

<p>
<a href="./documentation/guides/rust#readme">
<img alt="Ockam" src="./documentation/concepts/assets/ockam-features.png" width="900">
</a>
</p>

To learn more please see our [step-by-step guide](./documentation/guides/rust#readme).

## Next Steps

* [__Build End-to-End Encryption with Rust__](./documentation/use-cases/end-to-end-encryption-with-rust#readme):
In this hands-on guide, we create two small Rust programs called Alice and Bob. Alice and Bob send each other
messages, over the network, via a cloud service. They mutually authenticate each other and have a cryptographic
guarantee that the integrity, authenticity, and confidentiality of their messages is protected end-to-end.
[👉](./documentation/use-cases/end-to-end-encryption-with-rust#readme)

* [__Build End-to-End Encryption through Kafka__](./documentation/use-cases/end-to-end-encryption-through-kafka#readme):
In this guide, we show two programs called Alice and Bob. Alice and Bob send each other messages, over
the network, via a cloud service, _through Kafka_. They mutually authenticate each other and have a
cryptographic guarantee that the integrity, authenticity, and confidentiality of their messages is protected
end-to-end. The Kafka instance, the intermediary cloud service and attackers on the network are not be able
to see or change the contents of en-route messages. The application data in Kafka is encrypted.
[👉](./documentation/use-cases/end-to-end-encryption-through-kafka#readme)

* [__How to end-to-end encrypt all application layer communication__](./documentation/use-cases/end-to-end-encrypt-all-application-layer-communication#readme):
In this hands-on guide, we'll create two simple Rust programs to __transparently tunnel__ arbitrary
application layer communication through Ockam's end-to-end encrypted, mutually authenticated secure channels.
These example programs are also available in a docker image so you can try them without setting up a rust
toolchain.
[👉](./documentation/use-cases/end-to-end-encrypt-all-application-layer-communication#readme)

* [__Build a secure access tunnel to a service in a remote private network__](./documentation/use-cases/secure-remote-access-tunnels#readme):
In this guide, we'll write a few simple Rust programs to programmatically create secure access tunnels to remote
services and devices that are running in a private network, behind a NAT. We'll then tunnel arbitrary communication
protocols through these secure tunnels.
[👉](./documentation/use-cases/secure-remote-access-tunnels#readme)

* [__Step-by-Step Deep Dive__](./documentation/guides/rust#readme):
In this step-by-step guide we write many small rust programs to understand the various building blocks
that make up Ockam. We dive into Node, Workers, Routing, Transport, Secure Channels and more.
[👉](./documentation/guides/rust#readme)

## License

The code in this repository is licensed under the terms of the [Apache License 2.0](LICENSE).

<hr>

<p>
<a href="https://github.com/ockam-network/ockam/actions?query=workflow%3A%22Continuous+Integration%22">
<img alt="Continuous Integration"
  src="https://github.com/ockam-network/ockam/workflows/Continuous%20Integration/badge.svg">
</a>

<a href="https://www.ockam.io/learn/how-to-guides/high-performance-team/conduct/">
<img alt="Contributor Covenant"
  src="https://img.shields.io/badge/Contributor%20Covenant-v2.0%20adopted-ff69b4.svg">
</a>

<hr>

<div align="center">
:sparkles:
<a href="./documentation/use-cases/end-to-end-encryption-with-rust#readme">
Hands-on Introduction: Build end-to-end encrypted, mutually-authenticated, secure messaging in Rust
</a>
:sparkles:
</div>
