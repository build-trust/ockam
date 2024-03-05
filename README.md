<a href="https://discord.gg/RAbjRr3kds"><img alt="Discord" src="https://img.shields.io/discord/1074960884490833952?label=Discord&logo=discord&style=flat&logoColor=white"></a>

# What is Ockam?

Ockam empowers you to build secure-by-design apps that can trust data-in-motion.

You can use Ockam to create end-to-end encrypted and mutually authenticated channels. Ockam secure channels authenticate using cryptographic identities and credentials. They give your apps granular control over all trust and access decisions. This control makes it easy to enforce fine-grained, attribute-based authorization policies – at scale.

These core capabilities are composed to enable private and secure communication in a wide variety of application architectures. For example, with one simple command an app in your cloud can create an encrypted portal to a micro-service in another cloud. The service doesn’t need to be exposed to the Internet. You don’t have to change anything about networks or firewalls.

```bash
# Create a TCP Portal Inlet to a Postgres server that is running in
# a remote private VPC in another cloud.
ockam tcp-inlet create --from 15432 --to postgres

# Access the Postgres server on localhost.
psql --host localhost --port 15432
```

<img width="1500" alt="An end-to-end encrypted portal to postgres" src="https://github.com/build-trust/ockam/assets/159583/d41da555-ce0d-4bdb-8462-35a00384ae63">

Similarly, using another simple command a kafka producer can publish end-to-end encrypted messages for a specific kafka consumer. Kafka brokers in the middle can’t see, manipulate, or accidentally leak sensitive enterprise data. This minimizes risk to sensitive business data and makes it easy to comply with data governance policies.

# Encrypted Portals

Portals carry various application protocols over end-to-end encrypted Ockam secure channels.

For example: a TCP Portal carries TCP over Ockam, a Kafka Portal carries Kafka Protocol over Ockam, etc. Since portals work with existing application protocols you can use them through companion Ockam Nodes, that run adjacent to your application, without changing any of your application’s code.

A tcp portal makes a remote tcp server virtually adjacent to the server’s clients. It has two parts: an inlet and an outlet. The outlet runs adjacent to the tcp server and inlets run adjacent to tcp clients. An inlet and the outlet work together to create a portal that makes the remote tcp server appear on localhost adjacent to a client. This client can then interact with this localhost server exactly like it would with the remote server. All communication between inlets and outlets is end-to-end encrypted.

<img width="1500" alt="Encrypted Portals" src="https://github.com/build-trust/ockam/assets/159583/7ef5c0de-3885-4ac6-b5ba-90956294c0ff">

You can use Ockam Command to start nodes with one or more inlets or outlets. The underlying [protocols](https://docs.ockam.io/reference/protocols) handle the hard parts — NATs are traversed; Keys are stored in vaults; Credentials are short-lived; Messages are authenticated; Data-integrity is guaranteed; Senders are protected from key compromise impersonation; Encryption keys are ratcheted; Nonces are never reused; Strong forward secrecy is ensured; Sessions recover from network failures; and a lot more.

# How does Ockam work?

Ockam is a stack of protocols to build secure-by-design apps that can trust data-in-motion. We provide a collection of programming libraries, command line tools, deployable components, and cloud services that make it simple for you to use these protocols within your apps. To understand how these protocols work together, please read our guide on [How does Ockam work?](https://docs.ockam.io/how-does-ockam-work)
