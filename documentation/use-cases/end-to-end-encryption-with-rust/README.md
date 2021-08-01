# End-to-End Encryption with Rust

In this hands-on guide, we'll create two small Rust programs called Alice and Bob. Alice and Bob
will send each other messages, over the network, via a cloud service.

In our [code example](#rust-example), Alice and Bob will mutually authenticate each other and will be
guaranteed that the _integrity, authenticity, and confidentiality_ of their messages is _protected end-to-end_.
The intermediary cloud service and attackers on the network will not be able to see or change the contents
of en-route messages. In later examples we'll also see how we can have this end-to-end protection even
when the communication path between Alice and Bob is more complex - with multiple transport connections,
a variety of transport protocols and many intermediaries.

### Remove implicit trust in porous network boundaries

Modern distributed applications operate in highly dynamic environments. Infrastructure automation,
microservices in multiple clouds or data centers, a mobile workforce, the Internet of Things, and Edge
computing mean that machines and applications are continuously leaving and entering network boundaries.
Application architects have learnt that they must lower the amount of trust they place in network boundaries
and infrastructure.

The vulnerability surface of our application cannot include _all code_ that may be running within the same
porous network boundary. That surface is too big, to dynamic and usually outside the control of an application
developer. Applications must instead take control of the security and reliability of their own data. To
do this, all messages that are received over the network must prove who sent them and show that they weren't
tampered or forged.

### Lower trust in intermediaries

Another aspect of modern applications that can take away Alice's and Bob's ability to rely on the integrity
and authenticity of incoming messages is intermediary services (like the cloud service in our example below).

Data, within distributed applications, are rarely exchanged over a single point-to-point transport connection.
Application messages routinely flow over complex, multi-hop, multi-protocol routes
— _across data centers, through queues and caches, via gateways and brokers_ —
before reaching their end destination.

Typically, when information or commands are exchanged through an intermediary service, the intermediary
is able to `READ` the messages that are being exchanged, `UPDATE` en-route messages,
`CREATE` messages that were never sent, and `DELETE` (never deliver) messages that were actually sent.
Alice and Bob are entirely dependent on the security of such intermediaries. If the defenses of an intermediary
are compromised, our application is also compromised.

Transport layer security protocols are unable to protect application messages because their protection
is limited by the length and duration of the underlying transport connection. If there is an intermediary
between Alice and Bob, the transport connection between Alice and the intermediary is completely different
from the transport connection between Bob and the intermediary. This is why the intermediary has full `CRUD`
permissions on the messages in motion.

In environments like _Microservices, Internet-of-Things, and Edge Computing_ there are usually many such
intermediaries. Our application’s vulnerability surface quickly grows and becomes unmanageable.

### Mutually Authenticated, End-to-End Encrypted Secure Channels with Ockam

Ockam is a suite of programming libraries that make it simple, for applications, to easily create any
number of lightweight, mutually-authenticated, end-to-end encrypted secure channels. These channels use
cryptography to guarantee end-to-end integrity, authenticity, and confidentiality of messages.

An application can use Ockam Secure Channels to enforce _least-privileged access_ to commands, data,
configuration, and software updates that are flowing, as messages, between its distributed parts. All code
running within the same network boundary and intermediary services no longer have implicit `CRUD` permissions
on our application's messages. Any tampering or forgery of messages is immediately detected.

_The vulnerability surface, of our application, becomes strikingly small._

### Rust Example

Let's build end-to-end protected communication between Alice and Bob, through a cloud service:

## Setup

* Install Rust

    ```
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
    ```

* Setup a new cargo project to get started.

    ```
    cargo new --lib hello_ockam && cd hello_ockam && mkdir examples \
      && echo 'ockam = "*"' >> Cargo.toml && cargo build
    ```

    If the above instructions don't work on your machine, please
    [post a question](https://github.com/ockam-network/ockam/discussions/1642),
    we would love to help.

## Bob

Create a file at `examples/bob.rs` and copy the below code snippet to it.

```rust
// examples/bob.rs

use ockam::{Context, Entity, Result, SecureChannels, TrustEveryonePolicy, Vault};
use ockam::{RemoteForwarder, Routed, TcpTransport, Worker, TCP};

struct Echoer;

#[ockam::worker]
impl Worker for Echoer {
    type Context = Context;
    type Message = String;

    async fn handle_message(&mut self, ctx: &mut Context, msg: Routed<String>) -> Result<()> {
        println!("\n[✓] Address: {}, Received: {}", ctx.address(), msg);

        // Echo the message body back on its return_route.
        ctx.send(msg.return_route(), msg.body()).await
    }
}

#[ockam::node]
async fn main(ctx: Context) -> Result<()> {
    // Initialize the TCP Transport.
    TcpTransport::create(&ctx).await?;

    // Create a Vault to safely store secret keys for Bob.
    let vault = Vault::create(&ctx)?;

    // Create an Entity to represent Bob.
    let mut bob = Entity::create(&ctx, &vault)?;

    // Create a secure channel listener for Bob that will wait for requests to
    // initiate an Authenticated Key Exchange.
    bob.create_secure_channel_listener("listener", TrustEveryonePolicy)?;

    // The computer that is running this program is likely within a private network and
    // not accessible over the internet.
    //
    // To allow Alice and others to initiate an end-to-end secure channel with this program
    // we connect with 1.node.ockam.network:4000 as a TCP client and ask the forwarding
    // service on that node to create a forwarder for us.
    //
    // All messages that arrive at that forwarding address will be send to this program
    // using the TCP connection we created as a client.
    let node_in_hub = (TCP, "1.node.ockam.network:4000");
    let forwarder = RemoteForwarder::create(&ctx, node_in_hub, "listener").await?;
    println!("\n[✓] RemoteForwarder was created on the node at: 1.node.ockam.network:4000");
    println!("Forwarding address of Bob's secure channel listener is:");
    println!("{}", forwarder.remote_address());

    // Start a worker, of type Echoer, at address "echoer".
    // This worker will echo back every message it receives, along its return route.
    ctx.start_worker("echoer", Echoer).await?;

    // We won't call ctx.stop() here, this program will run until you stop it with Ctrl-C
    Ok(())
}

```

Run Bob’s program:

```
cargo run --example bob
```

This program:

1. Initializes an Ockam Node and a TCP transport.
2. Creates an Entity to represent Bob.
3. As Bob, starts a Secure Channel Listener to accept request to begin an Authenticated Key Exchange.
4. Creates a Remote Forwarder, for Bob's Secure Channel Listener, on the cloud node at `1.node.ockam.network`.
5. Prints the Secure Channel Listener's forwarding address.
6. Starts and Echoer worker that prints any message it receives and echoes it back on its return route.

## Alice

Create a file at `examples/alice.rs` and copy the below code snippet to it.

```rust
// examples/alice.rs

use ockam::{route, Context, Entity, Result, SecureChannels, TrustEveryonePolicy, Vault};
use ockam::{TcpTransport, TCP};
use std::io;

#[ockam::node]
async fn main(mut ctx: Context) -> Result<()> {
    // Initialize the TCP Transport.
    TcpTransport::create(&ctx).await?;

    // Create a Vault to safely store secret keys for Alice.
    let vault = Vault::create(&ctx)?;

    // Create an Entity to represent Alice.
    let mut alice = Entity::create(&ctx, &vault)?;

    // This program expects that Bob has setup a forwarding address,
    // for his secure channel listener, on the Ockam node at 1.node.ockam.network:4000.
    //
    // From standard input, read this forwarding address for Bob's secure channel listener.
    println!("\nEnter the forwarding address of Bob's secure channel listener: ");
    let mut address = String::new();
    io::stdin().read_line(&mut address).expect("Error reading from stdin.");
    let forwarding_address = address.trim();

    // Combine the tcp address of the node and the forwarding_address to get a route
    // to Bob's secure channel listener.
    let route_to_bob_listener = route![(TCP, "1.node.ockam.network:4000"), forwarding_address];

    // As Alice, connect to Bob's secure channel listener, and perform an
    // Authenticated Key Exchange to establish an encrypted secure channel with Bob.
    let channel = alice.create_secure_channel(route_to_bob_listener, TrustEveryonePolicy)?;

    println!("\n[✓] End-to-end encrypted secure channel was established.\n");

    loop {
        // Read a message from standard input.
        println!("Type a message for Bob's echoer:");
        let mut message = String::new();
        io::stdin().read_line(&mut message).expect("Error reading from stdin.");
        let message = message.trim();

        ctx.send(route![channel.clone(), "echoer"], message.to_string()).await?;

        // Wait to receive a reply and print it.
        let reply = ctx.receive::<String>().await?;
        println!("Alice received an echo: {}\n", reply); // should print "Hello Ockam!"
    }

    // This program will keep running until you stop it with Ctrl-C
}

```

Run Alice's program:

```
cargo run --example alice
```

This program:

1. Initializes an Ockam Node and a TCP transport.
2. Creates an Entity to represent Alice.
3. Waits to accept as input forwarding address of Bob's Secure Channel Listener.
4. Initiates an end-to-end secure channel with Bob via his forwarding address on a cloud node.

<div style="display: none; visibility: hidden;">
<hr><b>Next:</b> <a href="../../guides/rust#readme">A step-by-step introduction</a>
</div>
