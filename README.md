<a href="https://discord.gg/RAbjRr3kds"><img alt="Discord" src="https://img.shields.io/discord/1074960884490833952?label=Discord&logo=discord&style=flat&logoColor=white"></a>

# Trust for Data-in-Motion

Ockam is a suite of open source programming libraries and command line tools to
orchestrate end-to-end encryption, mutual authentication, key management, credential
management, and authorization policy enforcement – at massive scale.

Modern applications are distributed and have an unwieldy number of
interconnections that must trustfully exchange data. To trust data-in-motion,
applications need end-to-end guarantees of data authenticity, integrity, and
confidentiality. To be private and secure by-design, applications must have
granular control over every trust and access decision. Ockam allows you to add
these controls and guarantees to any application.

## Quick Start

Let's build a solution for a very common secure communication topology that
applies to many real world use cases. We'll build our first example using
[Ockam Command](https://docs.ockam.io/reference/command) but it is just as easy
to build end-to-end trustful communication using
[Ockam Programming Libraries](https://docs.ockam.io/reference/libraries/rust)

An application service and an application client running in two private networks
wish to securely communicate with each other without exposing ports on the
Internet. In a few simple commands, we’ll make them safely talk to each other
through an End-to-End Encrypted Cloud Relay.

## Install Ockam Command

If you use Homebrew, you can install Ockam using brew.

```bash
# Tap and install Ockam Command
brew install build-trust/ockam/ockam
```

This will download a precompiled binary and add it to your path.
If you don’t use Homebrew, you can also install on Linux and MacOS systems using curl.

```bash
curl --proto '=https' --tlsv1.2 -sSfL https://install.command.ockam.io | bash
```

### End-to-end encrypted and mutually authenticated communication

Next, step through the following commands to setup secure and private
communication between our application service and an application client.

```bash
# Check that everything was installed correctly by enrolling with Ockam Orchestrator.
#
# This will create a Space and Project for you in Ockam Orchestrator and provision an
# End-to-End Encrypted Cloud Relay service in your `default` project at `/project/default`.
ockam enroll

# -- APPLICATION SERVICE --

# Start an application service, listening on a local IP and port, that clients would access
# through the cloud encrypted relay. We'll use a simple HTTP server for this first example
# but this could be any other application service.
python3 -m http.server --bind 127.0.0.1 6000

# In a new terminal window, setup a tcp-outlet that makes a TCP service available at the given
# address `6000`. We can use this to send raw TCP traffic to the HTTP server on port `6000`.
# Finally create a relay in your default Orchestrator project. Relays make it possible to
# establish end-to-end protocols with services operating in remote private networks, without
# requiring a remote service to expose listening ports to an outside hostile network like the
# Internet.
ockam tcp-outlet create --to 6000
ockam relay create

# -- APPLICATION CLIENT --

# Setup a local tcp-inlet to allow raw TCP traffic to be received on port `7000` before
# it is forwarded. A TCP inlet is a way of defining where a node should be listening for
# connections, and where it should forward that traffic to.
ockam tcp-inlet create --from 7000

# Access the application service, that may be in a remote private network though
# the end-to-end encrypted secure channel, via your private and encrypted cloud relay.
curl --head 127.0.0.1:7000
```

### Private and secure by design

In the example above, we’ve created two nodes and established an end-to-end
secure channel between them through an encrypted cloud relay. For the sake of
simplicity, we ran both ends on a single machine but they could also be run on
completely separate machines with the same result: an end-to-end encrypted and
mutually authenticated secure channel.

Distributed applications that are connected in this way can communicate without
the risk of spoofing, tampering, or eavesdropping attacks irrespective of transport
protocols, communication topologies, and network configuration. As application
data flows across data centers, through queues and caches, via gateways and
brokers - these intermediaries, like the cloud relay in the above example, can
facilitate communication but cannot eavesdrop or tamper data.

You can establish secure channels across networks and clouds over multi-hop,
multi-protocol routes to build private and secure by design distributed applications
that have a small vulnerability surface and full control over data authenticity,
integrity, and confidentiality.

### Trust for data-in-motion

Behind the scenes, the above commands generated unique cryptographically
provable identities and saved corresponding keys in a vault. Your orchestrator
project was provisioned with a managed credential authority and every node was
setup to anchor trust in credentials issued by this authority. Identities were
issued project membership credentials and these cryptographically verifiable
credentials were then combined with attribute based access control policies to
setup a mutually authenticated and authorized end-to-end secure channel.

Your applications can make granular access control decisions at every request
because they can be certain about the source and integrity of all data and instructions.
You place zero implicit trust in network boundaries and intermediaries to build
applications that have end-to-end application layer trust for all data in motion.

### Powerful protocols, made simple

Underlying all of this is a variety of cryptographic and messaging protocols.
We’ve made these protocols safe and easy to use in any application.

No more having to think about creating unique cryptographic keys and issuing
credentials to all application entities. No more designing ways to safely store
secrets in hardware and securely distribute roots of trust. Ockam’s integrated
approach takes away this complexity and gives you simple tools for:

<ins>End-to-end data authenticity, integrity, and privacy in any communication topology</ins>

* Create end-to-end encrypted, authenticated secure channels over any transport topology.
* Create secure channels over multi-hop, multi-protocol routes - TCP, UDP, WebSockets, BLE, etc.
* Provision encrypted relays for applications distributed across many edge and cloud private networks.
* Make legacy protocols secure by tunneling them through mutually authenticated and encrypted portals.
* Bring end-to-end encryption to enterprise messaging, pub/sub and event streams - Kafka, RabbitMQ etc.

<ins>Identity-based, policy driven, application layer trust – granular authentication and authorization</ins>

* Generate cryptographically provable unique identities.
* Store private keys in safe vaults - hardware secure enclaves and cloud key management systems.
* Operate scalable credential authorities to issue lightweight, short-lived, attribute-based credentials.
* Onboard fleets of self-sovereign application identities using secure enrollment protocols.
* Rotate and revoke keys and credentials – at scale, across fleets.
* Define and enforce project-wide attribute based access control policies - ABAC, RBAC or ACLs.
* Integrate with enterprise identity providers and policy providers for seamless employee access.

## Deep Dives

Next let's dive into a step-by-step guide on our command line and programming libraries.

* [__Ockam Command__](https://docs.ockam.io/reference/command)
Command line tools to build and orchestrate highly secure distributed applications.
Orchestrate nodes, vaults, identities, credentials, secure channels, relays, portals and more.
[👉](https://docs.ockam.io/reference/command)

* [__Ockam Programming Libraries__](https://docs.ockam.io/reference/libraries)
Rust, Elixir and soon Typescript libraries to build secure by design applications for any environment
– from highly scalable cloud infrastructure to tiny battery operated microcontroller based devices.
[👉](https://docs.ockam.io/reference/libraries)

## License

The code in this repository is licensed under the terms of the [Apache License 2.0](LICENSE).

## Learn more about Ockam

- [ockam.io](https://www.ockam.io/)
- [Documentation](https://docs.ockam.io/)
- [Contribute to Ockam](https://github.com/build-trust/.github/blob/main/CONTRIBUTING.md#contributing-to-ockam-on-github)
- [Build Trust community Discord server](https://discord.gg/RAbjRr3kds)
- [Ockam Orchestrator on AWS Marketplace](https://aws.amazon.com/marketplace/pp/prodview-wsd42efzcpsxk)
