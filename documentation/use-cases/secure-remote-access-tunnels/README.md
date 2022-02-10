# Build a secure access tunnel to a service in a remote private network

In this guide, we'll write a few simple [Rust programs](#setup) to programmatically create
**secure access tunnels to remote services and devices that are running in a private network**,
behind a NAT. We'll then tunnel arbitrary communication protocols through these secure tunnels.

Ockam is a suite of open source programming libraries that make it easy to create lightweight
secure channels over complex, multi-hop, multi-protocol transport routes. When we combine
*Ockam Secure Channels* with *Ockam Routing* and *Transport Inlets/Outlets*, we can
**transparently tunnel** arbitrary protocols through Ockam's end-to-end encrypted and mutually
authenticated secure channels.

This gives us a highly composable set of building blocks that can enable secure access in a
wide variety of communication topologies - channels can extend to non-IP protocols, channels have
end-to-end trust guarantees and channels can be tunneled inside other channels.

With this approach, we can easily create secure remote access tunnels to enterprise services
in a private data center, machines in a factory, health devices in a hospital, microservices in a
Kubernetes cluster, or even an in-development web-service running on your laptop at home.

<p><img alt="Secure Remote Access Tunnels using Ockam" src="./diagrams/04.png"></p>

[Show me the code](#setup)

The ability to monitor, control, and fix issues in remote machines, applications, and services
is critical for many businesses. However, to make remote access possible, teams often end up
exposing their network by opening ports to the Internet.

The 2020 Unit 42 Incident Response and Breach Report studied 1000 ransomware incidents and found
that **in 50% of ransomware cases the initial attack vector was - exposed remote desktop access**.
In February 2021, an attack tried to poison the water supply of a town in Florida using Internet
exposed TeamViewer access.

Traditional approaches, like VPNs, are often not used because they are extremely hard to provision,
configure, and operate securely. They require complex credential management, are unreliable if your
machines only have intermittent connectivity, and don't work for non-IP protocols that are common
in industrial environments.

Applications that are protected only by a VPN boundary have a very large vulnerability surface.
This surface includes _all code_ running within that VPN boundary. VPNs have no way to granularly
authorize access to specific applications. If any one application within a VPN is compromised an
attacker gets unfettered ability to laterally move within that boundary and compromise more systems
and services.

A much safer approach is to **dynamically create policy-driven, short-lived, granularly authorized,
least-privileged access to specific remote services**. The below examples show how we can do
this, using Ockam, in a few lines of Rust.

Each example below incrementally builds on the examples before it, only a few lines of new code
is introduced in each example.

## Setup

If you don't have it, please [install](https://www.rust-lang.org/tools/install) the latest version of Rust.

```
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

Next, create a new cargo project to get started:

```
cargo new --lib secure_remote_access && cd secure_remote_access && mkdir examples &&
  echo 'ockam = "*"' >> Cargo.toml && cargo build
```

If the above instructions don't work on your machine please
[post a question](https://github.com/ockam-network/ockam/discussions/1642),
we would love to help.

## 01: Setup an Inlet and an Outlet

<p><img alt="Secure Remote Access using Ockam" src="./diagrams/01.png"></p>

In our first example, let's create a TCP Inlet and Outlet pair.

A **TCP Outlet** starts up as a **TCP client** to a given target TCP server address. It opens
a TCP connection with the target and then waits to receive Ockam Routing messages from an Inlet.
The payload of an incoming Ockam Routing message is converted into raw TCP and sent over the outlet
connection to the target. Any replies from the TCP target are wrapped as payloads of new Ockam
Routing messages and routed to the Inlet.

A **TCP Inlet** starts up as a **TCP server** and waits for incoming TCP connections. It knows the
route to a corresponding outlet. When new data arrives from a TCP client, the Inlet wraps this data
as payload of a new Ockam Routing message and sends it to the Outlet.

Create a file at `examples/01-inlet-outlet.rs` and copy the below code snippet to it.

```rust
// examples/01-inlet-outlet.rs
use ockam::{route, Context, Result, TcpTransport};

#[ockam::node]
async fn main(ctx: Context) -> Result<()> {
    // Initialize the TCP Transport.
    let tcp = TcpTransport::create(&ctx).await?;

    // Expect second command line argument to be the TCP address of a target TCP server.
    // For example: 127.0.0.1:5000
    //
    // Create a TCP Transport Outlet - at Ockam Worker address "outlet" -
    // that will connect, as a TCP client, to the target TCP server.
    //
    // This Outlet will:
    // 1. Unwrap the payload of any Ockam Routing Message that it receives from an Inlet
    //    and send it as raw TCP data to the target TCP server. First such message from
    //    an Inlet is used to remember the route back the Inlet.
    //
    // 2. Wrap any raw TCP data it receives, from the target TCP server,
    //    as payload of a new Ockam Routing Message. This Ockam Routing Message will have
    //    its onward_route be set to the route to an Inlet, that it knows about, because of
    //    a previous message from the Inlet.

    let outlet_target = std::env::args().nth(2).expect("no outlet target given");
    tcp.create_outlet("outlet", outlet_target).await?;

    // Expect first command line argument to be the TCP address on which to start an Inlet
    // For example: 127.0.0.1:4001
    //
    // Create a TCP Transport Inlet that will listen on the given TCP address as a TCP server.
    //
    // The Inlet will:
    // 1. Wrap any raw TCP data it receives from a TCP client as payload of a new
    //    Ockam Routing Message. This Ockam Routing Message will have its onward_route
    //    be set to the route to a TCP Transport Outlet. This route is provided as the 2nd
    //    argument of the create_inlet() function.
    //
    // 2. Unwrap the payload of any Ockam Routing Message it receives back from the Outlet
    //    and send it as raw TCP data to a connected TCP client.

    let inlet_address = std::env::args().nth(1).expect("no inlet address given");
    tcp.create_inlet(inlet_address, route!["outlet"]).await?;

    // We won't call ctx.stop() here,
    // so this program will keep running until you interrupt it with Ctrl-C.
    Ok(())
}

```

Before running the example program, start a target TCP server listening on port `5000`. As a first
example use a simple HTTP server, later we'll try other TCP-based protocols.

```
pushd $(mktemp -d 2>/dev/null || mktemp -d -t 'tmpdir') &>/dev/null; python3 -m http.server --bind 0.0.0.0 5000; popd
```

The example program takes two arguments. The first argument is the TCP address on which to start an Inlet
(port `4001`) and the second argument is the TCP address of our target TCP server (port `5000`).

```
cargo run --example 01-inlet-outlet 127.0.0.1:4001 127.0.0.1:5000
```

Now run an HTTP client, but instead of pointing it directly to our HTTP server, make a request to
the Inlet at port `4001`.

```
curl http://127.0.0.1:4001
```

When we run this, we see the data flow as shown in the [diagram above](#01-setup-an-inlet-and-an-outlet) -
HTTP requests and responses are wrapped in Ockam Routing messages and tunneled through our simple Rust program.

## 02: Route over a Transport

<p><img alt="Secure Remote Access using Ockam" src="./diagrams/02.png"></p>

Next let's separate the Inlet and Outlet from [example 01](#01-setup-an-inlet-and-an-outlet) into
two programs connected using the Ockam TCP Transport. An Ockam Transport carries Ockam Routing messages
from one machine to another machine. An Ockam Node is any program that communicates with other Ockam Nodes
using Ockam Routing and Transports.

The next two code snippets show how we can create such inlet and outlet nodes and tunnel
HTTP, over TCP, through them.

Create a file at `examples/02-outlet.rs` and copy the below code snippet to it.

```rust
// examples/02-outlet.rs
use ockam::{Context, Result, TcpTransport};

#[ockam::node]
async fn main(ctx: Context) -> Result<()> {
    // Initialize the TCP Transport.
    let tcp = TcpTransport::create(&ctx).await?;

    // Expect first command line argument to be the TCP address of a target TCP server.
    // For example: 127.0.0.1:5000
    //
    // Create a TCP Transport Outlet - at Ockam Worker address "outlet" -
    // that will connect, as a TCP client, to the target TCP server.
    //
    // This Outlet will:
    // 1. Unwrap the payload of any Ockam Routing Message that it receives from an Inlet
    //    and send it as raw TCP data to the target TCP server. First such message from
    //    an Inlet is used to remember the route back the Inlet.
    //
    // 2. Wrap any raw TCP data it receives, from the target TCP server,
    //    as payload of a new Ockam Routing Message. This Ockam Routing Message will have
    //    its onward_route be set to the route to an Inlet that is knows about because of
    //    a previous message from the Inlet.

    let outlet_target = std::env::args().nth(1).expect("no outlet target given");
    tcp.create_outlet("outlet", outlet_target).await?;

    // Create a TCP listener to receive Ockam Routing Messages from other ockam nodes.
    tcp.listen("127.0.0.1:4000").await?;

    // We won't call ctx.stop() here,
    // so this program will keep running until you interrupt it with Ctrl-C.
    Ok(())
}

```

Create a file at `examples/02-inlet.rs` and copy the below code snippet to it.

```rust
// examples/02-inlet.rs
use ockam::{route, Context, Result, TcpTransport, TCP};

#[ockam::node]
async fn main(ctx: Context) -> Result<()> {
    // Initialize the TCP Transport.
    let tcp = TcpTransport::create(&ctx).await?;

    // We know network address of the node with an Outlet, we also know that Outlet lives at "outlet"
    // address at that node.

    let route_to_outlet = route![(TCP, "127.0.0.1:4000"), "outlet"];

    // Expect first command line argument to be the TCP address on which to start an Inlet
    // For example: 127.0.0.1:4001
    //
    // Create a TCP Transport Inlet that will listen on the given TCP address as a TCP server.
    //
    // The Inlet will:
    // 1. Wrap any raw TCP data it receives from a TCP client as payload of a new
    //    Ockam Routing Message. This Ockam Routing Message will have its onward_route
    //    be set to the route to a TCP Transport Outlet. This route_to_outlet is provided as
    //    the 2nd argument of the create_inlet() function.
    //
    // 2. Unwrap the payload of any Ockam Routing Message it receives back from the Outlet
    //    and send it as raw TCP data to q connected TCP client.

    let inlet_address = std::env::args().nth(1).expect("no inlet address given");
    tcp.create_inlet(inlet_address, route_to_outlet).await?;

    // We won't call ctx.stop() here,
    // so this program will keep running until you interrupt it with Ctrl-C.
    Ok(())
}

```

Before we can run our example, let's start a target HTTP server listening on port `5000`.

```
pushd $(mktemp -d 2>/dev/null || mktemp -d -t 'tmpdir') &>/dev/null; python3 -m http.server --bind 0.0.0.0 5000; popd
```

Next start the outlet program and give it the address of the target TCP server:

```
cargo run --example 02-outlet 127.0.0.1:5000
```

Then start the inlet program and give it the TCP address on which the Inlet will wait for incoming TCP connections

```
cargo run --example 02-inlet 127.0.0.1:4001
```

Now run an HTTP client, but instead of pointing it directly to our HTTP server, make a request to
the Inlet at port `4001`.

```
curl http://127.0.0.1:4001
```

When we run this, we see the data flow as shown in the [diagram above](#02-route-over-a-transport) -
HTTP requests and responses are wrapped in Ockam Routing messages, carried over TCP, unwrapped and
then delivered to the target HTTP server.

## 03: Tunnel through a Secure Channel

<p><img alt="Secure Remote Access using Ockam" src="./diagrams/03.png"></p>

Next let's add an end-to-end encrypted and mutually authenticated secure channel to
[example 02](#02-route-over-a-transport). The rest of the code will stay the same.

For the remote access use-case, our outlet program is running the TCP listener at port `4000`. To
make the communication between our two nodes secure, we'll also make it run a secure channel listener
at address: `secure_channel_listener_service`.

The inlet program will then initiate a secure channel handshake over the route:
```
route![(TCP, "127.0.0.1:4000"), "secure_channel_listener_service"]
```

Create a file at `examples/03-outlet.rs` and copy the below code snippet to it.

```rust
// examples/03-outlet.rs
use ockam::{Context, Result, TcpTransport};
use ockam::{Identity, TrustEveryonePolicy, Vault};

#[ockam::node]
async fn main(ctx: Context) -> Result<()> {
    // Initialize the TCP Transport.
    let tcp = TcpTransport::create(&ctx).await?;

    // Create:
    //   1. A Vault to store our cryptographic keys
    //   2. An Identity to represent this Node
    //   3. A Secure Channel Listener at Worker address - secure_channel_listener_service
    //      that will wait for requests to start an Authenticated Key Exchange.

    let vault = Vault::create();
    let mut e = Identity::create(&ctx, &vault).await?;
    e.create_secure_channel_listener("secure_channel_listener_service", TrustEveryonePolicy).await?;

    // Expect first command line argument to be the TCP address of a target TCP server.
    // For example: 127.0.0.1:5000
    //
    // Create a TCP Transport Outlet - at Ockam Worker address "outlet" -
    // that will connect, as a TCP client, to the target TCP server.
    //
    // This Outlet will:
    // 1. Unwrap the payload of any Ockam Routing Message that it receives from an Inlet
    //    and send it as raw TCP data to the target TCP server. First such message from
    //    an Inlet is used to remember the route back the Inlet.
    //
    // 2. Wrap any raw TCP data it receives, from the target TCP server,
    //    as payload of a new Ockam Routing Message. This Ockam Routing Message will have
    //    its onward_route be set to the route to an Inlet that is knows about because of
    //    a previous message from the Inlet.

    let outlet_target = std::env::args().nth(1).expect("no outlet target given");
    tcp.create_outlet("outlet", outlet_target).await?;

    // Create a TCP listener to receive Ockam Routing Messages from other ockam nodes.
    tcp.listen("127.0.0.1:4000").await?;

    // We won't call ctx.stop() here,
    // so this program will keep running until you interrupt it with Ctrl-C.
    Ok(())
}

```

Create a file at `examples/03-inlet.rs` and copy the below code snippet to it.

```rust
// examples/03-inlet.rs
use ockam::{route, Context, Result, TcpTransport, TCP};
use ockam::{Identity, TrustEveryonePolicy, Vault};

#[ockam::node]
async fn main(ctx: Context) -> Result<()> {
    // Initialize the TCP Transport.
    let tcp = TcpTransport::create(&ctx).await?;

    // Create a Vault to store our cryptographic keys and an Identity to represent this Node.
    // Then initiate a handshake with the secure channel listener on the node that has the
    // TCP Transport Outlet.
    //
    // For this example, we know that the Outlet node is listening for Ockam Routing Messages
    // over TCP at "127.0.0.1:4000" and its secure channel listener is
    // at address: "secure_channel_listener_service".

    let vault = Vault::create();
    let mut e = Identity::create(&ctx, &vault).await?;
    let r = route![(TCP, "127.0.0.1:4000"), "secure_channel_listener_service"];
    let channel = e.create_secure_channel(r, TrustEveryonePolicy).await?;

    // We know Secure Channel address that tunnels messages to the node with an Outlet,
    // we also now that Outlet lives at "outlet" address at that node.

    let route_to_outlet = route![channel, "outlet"];

    // Expect first command line argument to be the TCP address on which to start an Inlet
    // For example: 127.0.0.1:4001
    //
    // Create a TCP Transport Inlet that will listen on the given TCP address as a TCP server.
    //
    // The Inlet will:
    // 1. Wrap any raw TCP data it receives from a TCP client as payload of a new
    //    Ockam Routing Message. This Ockam Routing Message will have its onward_route
    //    be set to the route to a TCP Transport Outlet. This route_to_outlet is provided as
    //    the 2nd argument of the create_inlet() function.
    //
    // 2. Unwrap the payload of any Ockam Routing Message it receives back from the Outlet
    //    and send it as raw TCP data to q connected TCP client.

    let inlet_address = std::env::args().nth(1).expect("no inlet address given");
    tcp.create_inlet(inlet_address, route_to_outlet).await?;

    // We won't call ctx.stop() here,
    // so this program will keep running until you interrupt it with Ctrl-C.
    Ok(())
}

```

Before we can run our example, let's start a target HTTP server listening on port `5000`.

```
pushd $(mktemp -d 2>/dev/null || mktemp -d -t 'tmpdir') &>/dev/null; python3 -m http.server --bind 0.0.0.0 5000; popd
```

Next start the outlet program and give it the address of the target TCP server:

```
cargo run --example 03-outlet 127.0.0.1:5000
```

Then start the inlet program and give it the TCP address on which the Inlet will wait for incoming TCP connections

```
cargo run --example 03-inlet 127.0.0.1:4001
```

Now run an HTTP client, but instead of pointing it directly to our HTTP server, make a request to
the Inlet at port `4001`.

```
curl http://127.0.0.1:4001
```

When we run this, we see the data flow as shown in the [diagram above](#03-tunnel-through-a-secure-channel) -

* An HTTP request is wrapped in an Ockam Routing message and routed to the mutually authenticated
  secure channel.
* The channel encrypts this routing message (using an AEAD construction) and makes this encrypted
  message in-turn a payload of a brand new Ockam Routing message.
* This new message is routed over TCP to the other end of the channel where it is decrypted and
  checked for authenticity and integrity.
* This decrypted message is itself an Ockam Routing message destined for the Outlet.
* When it reaches the Outlet, the Outlet unwraps the HTTP request payload and sends it over TCP to our
  target HTTP server.

## 04: Remote Access via a Forwarder

<p><img alt="Secure Remote Access using Ockam" src="./diagrams/04.png"></p>

As a final step, let's create a Forwarder on an Ockam Node in public cloud.

To allow an Inlet Node to initiate an end-to-end secure channel with and the Outlet Node
which is not exposed to the Internet. We connect with an existing node at `1.node.ockam.network:4000`
as a TCP client and ask the forwarding service on that node to create a forwarder for us.

All messages that arrive at that forwarding address will be sent to this program using the TCP
connection we created as a client. The forwarding node only sees end-to-end encrypted data.
You can easily [create your own forwarding nodes](../../guides/rust#step-by-step), for this example we've
created one that live at `1.node.ockam.network:4000`.

We only need to change a few minor details of our program in
[example 03](#03-tunnel-through-a-secure-channel).

Create a file at `examples/04-outlet.rs` and copy the below code snippet to it.

```rust
// examples/04-outlet.rs
use ockam::{Context, RemoteForwarder, Result, TcpTransport, TCP};
use ockam::{Identity, TrustEveryonePolicy, Vault};

#[ockam::node]
async fn main(ctx: Context) -> Result<()> {
    // Initialize the TCP Transport.
    let tcp = TcpTransport::create(&ctx).await?;

    let vault = Vault::create();
    let mut e = Identity::create(&ctx, &vault).await?;
    e.create_secure_channel_listener("secure_channel_listener_service", TrustEveryonePolicy).await?;

    // Expect first command line argument to be the TCP address of a target TCP server.
    // For example: 127.0.0.1:5000
    //
    // Create a TCP Transport Outlet - at Ockam Worker address "outlet" -
    // that will connect, as a TCP client, to the target TCP server.
    //
    // This Outlet will:
    // 1. Unwrap the payload of any Ockam Routing Message that it receives from an Inlet
    //    and send it as raw TCP data to the target TCP server. First such message from
    //    an Inlet is used to remember the route back the Inlet.
    //
    // 2. Wrap any raw TCP data it receives, from the target TCP server,
    //    as payload of a new Ockam Routing Message. This Ockam Routing Message will have
    //    its onward_route be set to the route to an Inlet that is knows about because of
    //    a previous message from the Inlet.

    let outlet_target = std::env::args().nth(1).expect("no outlet target given");
    tcp.create_outlet("outlet", outlet_target).await?;

    // To allow Inlet Node and others to initiate an end-to-end secure channel with this program
    // we connect with 1.node.ockam.network:4000 as a TCP client and ask the forwarding
    // service on that node to create a forwarder for us.
    //
    // All messages that arrive at that forwarding address will be sent to this program
    // using the TCP connection we created as a client.
    let node_in_hub = (TCP, "1.node.ockam.network:4000");
    let forwarder = RemoteForwarder::create(&ctx, node_in_hub).await?;
    println!("\n[✓] RemoteForwarder was created on the node at: 1.node.ockam.network:4000");
    println!("Forwarding address in Hub is:");
    println!("{}", forwarder.remote_address());

    // We won't call ctx.stop() here,
    // so this program will keep running until you interrupt it with Ctrl-C.
    Ok(())
}

```

Create a file at `examples/04-inlet.rs` and copy the below code snippet to it.

```rust
// examples/04-inlet.rs
use ockam::{route, Context, Result, Route, TcpTransport, TCP};
use ockam::{Identity, TrustEveryonePolicy, Vault};

#[ockam::node]
async fn main(ctx: Context) -> Result<()> {
    // Initialize the TCP Transport.
    let tcp = TcpTransport::create(&ctx).await?;

    // Create a Vault to store our cryptographic keys and an Identity to represent this Node.
    // Then initiate a handshake with the secure channel listener on the node that has the
    // TCP Transport Outlet.
    //
    // For this example, we know that the Outlet node is listening for Ockam Routing Messages
    // through a Remote Forwarder at "1.node.ockam.network:4000" and its forwarder address
    // points to secure channel listener.
    let vault = Vault::create();
    let mut e = Identity::create(&ctx, &vault).await?;

    // Expect second command line argument to be the Outlet node forwarder address
    let forwarding_address = std::env::args().nth(2).expect("no outlet forwarding address given");
    let r = route![(TCP, "1.node.ockam.network:4000"), forwarding_address, "secure_channel_listener_service"];
    let channel = e.create_secure_channel(r, TrustEveryonePolicy).await?;

    // We know Secure Channel address that tunnels messages to the node with an Outlet,
    // we also now that Outlet lives at "outlet" address at that node.
    let route_to_outlet: Route = route![channel, "outlet"];

    // Expect first command line argument to be the TCP address on which to start an Inlet
    // For example: 127.0.0.1:4001
    //
    // Create a TCP Transport Inlet that will listen on the given TCP address as a TCP server.
    //
    // The Inlet will:
    // 1. Wrap any raw TCP data it receives from a TCP client as payload of a new
    //    Ockam Routing Message. This Ockam Routing Message will have its onward_route
    //    be set to the route to a TCP Transport Outlet. This route_to_outlet is provided as
    //    the 2nd argument of the create_inlet() function.
    //
    // 2. Unwrap the payload of any Ockam Routing Message it receives back from the Outlet
    //    and send it as raw TCP data to q connected TCP client.

    let inlet_address = std::env::args().nth(1).expect("no inlet address given");
    tcp.create_inlet(inlet_address, route_to_outlet).await?;

    // We won't call ctx.stop() here,
    // so this program will keep running until you interrupt it with Ctrl-C.
    Ok(())
}

```

Before we can run our example, let's start a local target HTTP server listening on port `5000`.

```
pushd $(mktemp -d 2>/dev/null || mktemp -d -t 'tmpdir') &>/dev/null; python3 -m http.server --bind 0.0.0.0 5000; popd
```

Next start the outlet program and give it the address of the local target server. It will print the
assigned forwarding address in the cloud node, copy it.

```
cargo run --example 04-outlet 127.0.0.1:5000
```

Then start the inlet program and give it the address on which the Inlet will wait for incoming TCP connections
along with the forwarding address.

```
cargo run --example 04-inlet 127.0.0.1:4001 FORWARDING_ADDRESS_PRINTED_BY_OUTLET_PROGRAM
```

Now run an HTTP client and make a request to the Inlet address that was printed by the outlet program.

```
curl http://127.0.0.1:4001
```

When we run this, we see the data flow as shown in the [diagram above](#04-remote-access-inlets-in-the-cloud).

Our target program can receive requests from the client over an end-to-end encrypted channel. The Outlet and
the Inlet nodes an both run in private networks without opening a listening port that exposes them to attacks
from the Internet.

We've got a secure remote access tunnel in 32 lines of Rust (excluding comments) 🥳.

So far, we've only tried sending HTTP traffic through our tunnels. Try other TCP-based tools and protocol -
SSH, netcat and various monitoring and logging protocols can all be tunneled in this way.

# Conclusion

The above pattern of matryoshka dolls like nesting of Ockam Routing messages inside other Ockam
Routing messages is incredibly powerful, lightweight and composable. It allows us to create
**tunnels - inside tunnels - inside tunnels.**

In the examples above, we saw a glimpse of the flexible set of
[composable building blocks](../../guides/rust#readme) available in the Ockam rust crates. These building
blocks can be easily combined for a variety of end-to-end secure, private,
and [trustful communication use-cases](https://github.com/ockam-network/ockam#next-steps).

Ockam's Identity and Trust features plug into existing workforce identity and enterprise access
policy engines to enable **dynamically created, policy-driven, short-lived, granularly authorized,
least-privileged access to specific remote services**.

This approach minimizes the vulnerability surface of remote access. Inlets can be opened just-in-time
based on dynamic business workflows and authorization policy decisions. Outlets only have access to one
specific target service.

To learn more, please see our [rust guide](../end-to-end-encryption-with-rust#readme) and
[step-by-step guide](../../guides/rust#readme).

[routing]: ../../guides/rust/get-started/03-routing#readme
[transports]: ../../guides/rust/get-started/04-transport#readme
[secure-channels]: ../../guides/rust/get-started/05-secure-channel#readme
