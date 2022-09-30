# Trust for Data-in-Motion.

Ockam is a suite of open source tools, programming libraries, and managed cloud services to orchestrate end-to-end encryption, mutual authentication, key management, credential management, and authorization policy enforcement – at massive scale.

Modern applications are distributed and have an unwieldy number of interconnections that must trustfully exchange data. To build trust for data-in-motion, applications need end-to-end guarantees of data authenticity, integrity, and confidentiality. To be private and secure by-design, applications must have granular control over every trust and access decision. Ockam allows you to add these controls and guarantees to any application.

Ockam was made for millions of builders. We are passionate about simple developer experiences and easy to use tools. If you can spin up EC2 or write data to a database from your application, then you are one of the millions of builders that already have the expertise to use Ockam.
Ockam empowers you to:

* Create end-to-end encrypted, authenticated Secure Channels over any transport topology.
* Provision Encrypted Relays for trustful communication within applications that are distributed across many edge, cloud and data-center private networks.
* Tunnel legacy protocols through mutually authenticated and encrypted Portals.
* Add-ons to bring end-to-end encryption to enterprise messaging, pub/sub and event streams.
* Generate unique cryptographically provable Identities and store private keys in safe Vaults. Add-ons for hardware or cloud key management systems.
* Operate project specific and scalable Credential Authorities to issue lightweight, short-lived, easy to revoke, attribute-based credentials.
* Onboard fleets of self-sovereign application identities using Secure Enrollment Protocols to issue credentials to application clients and services.
* Rotate and revoke keys and credentials – at scale, across fleets.
* Define and enforce project-wide Attribute Based Access Control (ABAC) policies.
* Add-ons to integrate with enterprise Identity Providers and Policy Providers.

# Get Started with Ockam Open Source

## 1. Install the Ockam CLI

### Homebrew

```bash
brew install build-trust/ockam/ockam
```

### Precompiled Binaries

```shell
curl --proto '=https' --tlsv1.2 -sSf https://raw.githubusercontent.com/build-trust/ockam/develop/install.sh | sh
```

## 2. Create a Local Relay

Let’s walk through a simple example to create an end-to-end encrypted, mutually authenticated, secure and private cloud relay – for any application.

First [install](../get-started/#command) the Ockam command, if you haven't already.

```bash
brew install build-trust/ockam/ockam
```

If you're on linux, see how to installed [precompiled binaries](../get-started/#precompiled-binaries).

Then let's create a local relay node.

```bash
ockam node create relay
```

## 3. Create an Application Service

Next let's prepare the service side of our application.

Start our application service, listening on a local ip and port, that clients would access through the cloud relay. We'll use a simple http server for our first example but this could be some other application service.

```bash
python3 -m http.server --bind 127.0.0.1 5000
```

Setup an Ockam node, called blue, as a sidecar next to our application service.

```
ockam node create blue
```

Create a tcp outlet on the blue node to send raw tcp traffic to the application service.

```bash
ockam tcp-outlet create --at /node/blue --from /service/outlet --to 127.0.0.1:5000
```

Then create a forwarding relay at your default orchestrator project to blue.

```bash
ockam forwarder create blue --at /node/relay --to /node/blue
```

## 4. Application Client

Now on the client side:

Setup an ockam node, called green, as a sidecar next to our application service.

```bash
ockam node create green
```

Then create an end-to-end encrypted secure channel with blue, through the cloud relay. Then tunnel traffic from a local tcp inlet through this end-to-end secure channel.

```bash
ockam secure-channel create --from /node/green \
     --to /node/relay/service/forward_to_blue/service/api \
  | ockam tcp-inlet create --at /node/green --from 127.0.0.1:7000 --to -/service/outlet
```

Access the application service though the end-to-end encrypted, secure relay.

```bash
curl 127.0.0.1:7000
```

We just created end-to-end encrypted, mutually authenticated, and authorized secure communication between a tcp client and server. This client and server can be running in separate private networks / NATs. We didn't have to expose our server by opening a port on the Internet or punching a hole in our firewall.

The two sides authenticated and authorized each other's known, cryptographically provable identifiers. In later examples we'll see how we can build granular, attribute-based access control with authorization policies.

## 5. Restart

If something breaks or if you'd like to start from the beginning as you try this example, please run:

```
ockam reset
```

## The Full Example

```bash
brew install build-trust/ockam/ockam
ockam node create relay

# -- APPLICATION SERVICE --

python3 -m http.server --bind 127.0.0.1 5000

ockam node create blue
ockam tcp-outlet create --at /node/blue --from /service/outlet --to 127.0.0.1:5000
ockam forwarder create blue --at /node/relay --to /node/blue

# -- APPLICATION CLIENT --

ockam node create green
ockam secure-channel create --from /node/green --to /node/relay/service/forward_to_blue/service/api \
  | ockam tcp-inlet create --at /node/green --from 127.0.0.1:7000 --to -/service/outlet

curl 127.0.0.1:7000
```

## Next Steps with the Rust Library

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

## Learn more about Ockam
[Ockam.io](https://www.ockam.io/)
[Documentation](https://docs.ockam.io/)
[Ockam Orchestrator on AWS Marketplace](https://aws.amazon.com/marketplace/pp/prodview-wsd42efzcpsxk)
