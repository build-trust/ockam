# Single Page Guide

## Setup

1. Install Rust

`curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`

2. Setup a hello_ockam Cargo Project to get started with Ockam

`cargo new --lib hello_ockam && cd hello_ockam && mkdir examples && echo 'ockam = "*"' >> Cargo.toml && cargo build`

For more details on the setup process, see the [Getting Started Guide]().

## Nodes

An Ockam Node is an asynchronous execution environment that can run very
lightweight, concurrent, stateful actors called Ockam Workers. A node can
deliver messages from one worker to another worker. Nodes can also route
messages to workers on other remote nodes.

A node requires an asynchronous runtime to concurrently execute workers.
The default Ockam Node implementation uses Tokio, a popular asynchronous
runtime in the Rust ecosystem. Over time, we plan to support Ockam Node
implementations for various `no_std` embedded targets.

The first thing any Ockam program must do is setup and start an Ockam node.
You could do these steps manually, but for convenience we provide an
`#[ockam::node]` attribute that injects all of this initialization.
It creates the asynchronous environment, initializes worker management,
sets up routing and initializes the node context.

Here we add the `#[ockam::node]` attribute to an `async` main function that
receives the node execution context as a parameter and returns `ockam::Result`
which helps make our error reporting better.

As soon as the main function starts, we use `ctx.stop()` to immediately stop
the node that was just started. If we don't add this line, the node will run
forever.

## Workers

Ockam Workers are lightweight, concurrent, stateful actors.

Workers:
* Run in an Ockam Node.
* Have an application-defined address (like a postal mail or email address).
* Can maintain internal state.
* Can start other new workers.
* Can handle messages from other workers running on the same or a different node.
* Can send messages to other workers running on the same or a different node.

Now that we've [created our first node](../01-node), let's create a new worker,
send it a message, and receive a reply.

To create a worker, a struct is created that can optionally have some fields
to store the worker's internal state. If the worker is stateless, it can be
defined as a field-less unit struct.

This struct:
* Must implement the `ockam::Worker` trait.
* Must have the `#[ockam::worker]` attribute on the Worker trait implementation
* Must define two associated types `Context` and `Message`
  * The `Context` type is usually set to `ockam::Context` which is provided by the node implementation.
  * The `Message` type must be set to the type of message the worker wishes to handle.

## Routing

The path that a message takes through an Ockam network is called a route. A
message carries route meta-data that nodes use to determine the next hop toward
the destination. A route is a list of worker addresses. The order of addresses
in a route defines the path the message will take from its source to its
destination.

A message has two routes: the **onward route** and the **return route**. The
onward route specifies the path the message takes to the destination. When
a node receives a message to route, the head of the address list is removed
from the route. This address is used to determine the next destination route,
or destination worker.

The return route of a message represents the path back to the source worker,
and may differ from the onward route. When a message is routed through a node,
the node adds its own address to the return route. This ensures that there is a
valid, known return path for message replies. All messages sent in an Ockam
Network have a route. Many messages between local workers have short routes,
only indicating the address of another local Worker.

## Transports

Ockam Transports are logical connections between Ockam Nodes. Ockam Transports
are an abstraction on top of physical transport protocols. The Ockam TCP
Transport is an implementation of an Ockam Transport using the TCP protocol.
This functionality is available in the `ockam_transport_tcp` crate, and is
included in the standard feature set of the top level `ockam` crate.

### Using the TCP Transport

The Ockam TCP Transport API fundamental type is `TcpTransport`. This type
provides the ability to create, connect, and listen for TCP connections. To
create a TCP transport, the Context is passed to the `create` function:

The return value of `create` is a handle to the transport itself, which is used
for `listen` calls. Listening on a local port is accomplished by
using the `listen` method. This method takes a string containing the IP address
and port, delimited by `:`. For example, this statement will listen on
localhost port 3000:

### Routing over Transports

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


To send a message to a worker on another node connected by a transport, the
address of the transport is added to the route first, followed by the address
of the destination worker.

## Entities

### Vaults and Entities

Ockam protocols like secure channels, key lifecycle, credential
exchange, and device enrollment depend on a variety of standard
cryptographic primitives or building blocks. Depending on the environment,
these building blocks may be provided by a software implementation or a
cryptographically capable hardware component.

To support a variety of security hardware, there is loose coupling between
Ockam security protocols' building blocks and the underlying specific hardware
implementation. This is achieved using an abstract notion called Vault. A
software vault worker implementation is available to Ockam nodes. Over time,
and with help from the Ockam open source community, we plan to add vaults for
several TEEs, TPMs, HSMs, and Secure Enclaves.

A vault is used by a top level worker called an entity. Entities offer a small,
simplified interface to complex security protocols. They provide the features
of the underlying protocols, while handling implementation details. The
interaction between multiple parties establishing trust is modeled using Entities.

Entities provide:
- Cryptographic key creation, rotation and retrieval
- Cryptographic proof creation and verification mechanism
- Secure Channel establishment
- Credential issuance and verification
- Change verification
- Contact management

## Profiles

A Profile is a specific identifier backed by a key pair. An Entity can have
multiple Profiles, by having multiple key pairs in the Vault.

The ability for an Entity to have multiple Profiles enhances the privacy of
an Entity. Two Profiles belonging to an Entity cannot be associated with one
another, or back to the Entity. This allows a single real user to use multiple
Profiles, each for a different identity scenario.

For example, a user may have a Manufacturer Identity for technical support, and
an Advertiser Identity for third party integrations.

Entities and Profiles implement the same APIs. In many Ockam APIs, Entities and
Profiles can be used interchangeably.

## Secure Channels

Secure channels are encrypted bi-directional message routes between two
entities. One entity acts as a listener in the secure channel protocol, and the
other is the initiator.

When a secure channel listener receives a request to start a new channel, it
checks a trust policy. A trust policy is a function that evaluates whether or
not an action is allowed. The trust policy that is checked when a secure
channel is created ensures that the requester is permitted to connect.

The secure channel creation protocol also verifies that the entity presenting a
profile identifier actually has possession of the keypair for that identifier.

The entities then run the key agreement protocol, which allows both entities to
securely agree upon a secret key without exchanging it over the network. The
algorithm used for key agreement is pluggable. By default, an implementation of
the Noise XX protocol is used. The Signal X3DH protocol is also available as a
crate add-on.

After the key agreement is done, the initiating entity starts a worker to
manage the secure channel. The address of this worker is used to send messages
through the channel. This address is included in routes just like transport and
other worker addresses.

### Creating a Secure Channel

Entities create a secure channel by calling `create_secure_channel_listener` on
the listening peer, and `Entity::create_secure_channel` on the initiating peer.

Creating the listener requires two parameters:
- The address of the secure channel being established
- A trust policy control on secure channel creation

Creating the initiator also requires two parameters:
- The route to the secure channel
- A trust policy control on secure channel creation

The `TrustEveryonePolicy` trust policy is used in the example below. This
policy will allow everyone to connect.

## Example: Echoer through Secure Channel

In this example, Alice creates a secure channel with Bob, through a middle hop.
Alice and Bob use the TCP transport to route messages through the middle hop.

### Example: Bob (Listener)

```rust
use fresh::Echoer;
use ockam::{
    Context, Entity, TrustEveryonePolicy, Result, SecureChannel, SecureChannels, TcpTransport, Vault,
};

#[ockam::node]
async fn main(mut ctx: Context) -> Result<()> {
  // Create an echoer worker
  ctx.start_worker("echoer", Echoer).await?;

  let vault = Vault::create(&ctx)?;
  let mut bob = Entity::create(&ctx, &vault)?;

  bob.create_secure_channel_listener("secure_channel_listener", TrustEveryonePolicy)?;

  // Initialize the TCP Transport.
  let tcp = TcpTransport::create(&ctx).await?;

  // Create a TCP listener and wait for incoming connections.
  tcp.listen("127.0.0.1:4000").await?;

  // Don't call ctx.stop() here so this node runs forever.
  Ok(())
}

```

### Example: Middle Node
```rust
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

### Example: Alice (Initiator)

```rust
use ockam::{ Context, Entity, TrustEveryonePolicy, Result, route, SecureChannels, TcpTransport, Vault, TCP };

#[ockam::node]
async fn main(mut ctx: Context) -> Result<()> {
  // Initialize the TCP Transport.
  let tcp = TcpTransport::create(&ctx).await?;

  let vault = Vault::create(&ctx)?;
  let mut alice = Entity::create(&ctx, &vault)?;

  // Connect to a secure channel listener and perform a handshake.
  let channel = alice.create_secure_channel(
    route![(TCP, "127.0.0.1:3000"),(TCP, "127.0.0.1:4000"),"secure_channel_listener"],
    TrustEveryonePolicy,
  )?;

  // Send a message to the echoer worker via the channel.
  ctx.send(route![channel, "echoer"], "Hello Ockam!".to_string())
        .await?;

  // Wait to receive a reply and print it.
  let reply = ctx.receive::<String>().await?;
  println!("App Received: {}", reply); // should print "Hello Ockam!"

  // Stop all workers, stop the node, cleanup and return. ctx.stop().await
}
```
