# Trust for Data-in-Motion.

Ockam is a suite of open source tools, programming libraries, and managed cloud
services to orchestrate end-to-end encryption, mutual authentication, key management,
credential management, and authorization policy enforcement – at massive scale.

Modern applications are distributed and have an unwieldy number of interconnections
that must trustfully exchange data. To build trust for data-in-motion, applications
need end-to-end guarantees of data authenticity, integrity, and confidentiality.
To be private and secure by-design, applications must have granular control over every
trust and access decision. Ockam allows you to add these controls and guarantees to any
application.

We are passionate about making powerful cryptographic and messaging protocols
__simple and safe to use__ for millions of builders.
For example, to create a mutually authenticated and end-to-end encrypted
secure channel between two Ockam nodes, all you have to do is:

```bash
$ ockam secure-channel create --from /node/n1 --to /node/n2/service/api \
    | ockam message send hello --from /node/n1 --to -/service/uppercase

HELLO
```

We handle all the underlying protocol complexity and provide secure, scalable, and reliable
building blocks for your applications. In the snippet above we used Ockam Command,
it's also just as easy to establish secure channels within your application code using our
[Rust Library](#next-steps-with-the-rust-library).

Ockam empowers you to:

* Create end-to-end encrypted, authenticated __Secure Channels__ over any transport topology.
* Provision __Encrypted Relays__ for trustful communication within applications that are
distributed across many edge, cloud and data-center private networks.
* Tunnel legacy protocols through mutually authenticated and encrypted __Portals__.
* Add-ons to bring end-to-end encryption to enterprise messaging, pub/sub and event streams.
* Generate unique cryptographically provable __Identities__ and store private keys in safe __Vaults__.
Add-ons for hardware or cloud key management systems.
* Operate project specific and scalable __Credential Authorities__ to issue lightweight, short-lived,
easy to revoke, attribute-based credentials.
* Onboard fleets of self-sovereign application identities using __Secure Enrollment Protocols__
to issue credentials to application clients and services.
* __Rotate__ and __revoke__ keys and credentials – at scale, across fleets.
* Define and enforce project-wide __Attribute Based Access Control (ABAC)__ policies.
* Add-ons to integrate with enterprise __Identity Providers__ and __Policy Providers__.
* Programming libraries for __Rust__, __Elixir__ and more on the roadmap.

# Get Started

## Install Ockam Command

If you use Homebrew, you can install Ockam using `brew`.

```bash
brew install build-trust/ockam/ockam
```

Otherwise, you can download our latest architecture specific pre-compiled binary by running:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://raw.githubusercontent.com/build-trust/ockam/develop/install.sh | sh
```

After the binary downloads, please move it to a location in your shell's `$PATH`, like `/usr/local/bin`.

# End-to-End Trustful communication using Relays.

Let's build a solution for a very common topology. A application service and an application client running
in two private networks wish to communicate with each other without exposing ports on the Internet.

``bash
# Create a relay node that will relay end-to-end encrypted messages
ockam node create relay

# -- APPLICATION SERVICE --

# Start our application service, listening on a local ip and port, that clients
# would access through the cloud relay. We'll use a simple http server for our
# first example but this could be some other application service.
python3 -m http.server --bind 127.0.0.1 5000

# Setup an ockam node, called blue, as a sidecar next to our application service.
# Create a tcp outlet on the blue node to send raw tcp traffic to the application service.
# Then create a forwarder on the relay node to blue.
ockam node create blue
ockam tcp-outlet create --at /node/blue --from /service/outlet --to 127.0.0.1:5000
ockam forwarder create blue --at /node/relay --to /node/blue

# -- APPLICATION CLIENT --

# Setup an ockam node, called green, as a sidecar next to our application client.
# Then create an end-to-end encrypted secure channel with blue, through the relay.
# Then tunnel traffic from a local tcp inlet through this end-to-end secure channel.
ockam node create green
ockam secure-channel create --from /node/green --to /node/relay/service/forward_to_blue/service/api \
  | ockam tcp-inlet create --at /node/green --from 127.0.0.1:7000 --to -/service/outlet

# Access the application service though the end-to-end encrypted, secure relay.
curl 127.0.0.1:7000
```

If something breaks or if you'd like to start from the beginning as you try this example, please run `ockam reset`.

In this example, we enabled two applications, a python web server and a
curl web client, to communicate with each other without exposing them to the internet and without
any change to their code. These two applications can run in two separate private networks and
communicate with each other over a relayed, end-to-end encrypted, mutually authenticated secure channel.

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

- [Ockam.io](https://www.ockam.io/)
- [Documentation](https://docs.ockam.io/)
- [Contribute to Ockam](https://github.com/build-trust/.github/blob/main/CONTRIBUTING.md#contributing-to-ockam-on-github)
- [Ockam Orchestrator on AWS Marketplace](https://aws.amazon.com/marketplace/pp/prodview-wsd42efzcpsxk)
