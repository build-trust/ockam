# End-to-end encrypted messaging with Kafka and Ockam Secure Channels

## Background

When using Apache Kafka and other stream processing services a common concern is data access and security.

Many fields, such as finance and personal information handling, have statutory requirements for data to be encrypted when stored or in transit.

Modern networks and data pipelines transfer data through multiple endpoints, often controlled by multiple vendors. It is therefore not sufficient for encryption to only cover the transit between arbitrary pairs of endpoints or once data comes to rest in storage.

This has led to the emergence of End-to-End encryption as the preferred model for messaging.

There were several attempts to define and implement end-to-end encryption for messaging system like Kafka, for example [KIP-317](https://cwiki.apache.org/confluence/display/KAFKA/KIP-317%3A+Add+end-to-end+data+encryption+functionality+to+Apache+Kafka)

Most of these attempts are using some sort of key storage and exchange persistent keys between devices. A fundamental weakness of such a system is its vulnerability to any party able to acquire the key.

Another approach would be to create a transient encryption key when establishing the session between two ends and have only them access it.

This approach is implemented in [Ockam Secure Channels](../../rust/06-secure-channel)

## Scenario

In this guide we're going to establish a Secure Channel and exchange messages via Apache Kafka using Ockam.

In order to establish a Secure Channel we need to be able to send messages between two ends bidirectionally. For that we are going to use two Kafka topics.

For simplicity we're going to use single partition topics.

Our goals are to make the message exchange:

- secure: no one except the endpoints can decrypt the messages
- reliable: messages are delivered eventually as long as the endpoints are alive

## The example

**TODO: do we want to have Rust code here, or git clone, or docker run???**


### Git clone

We will use a simple Rust program to demonstrate the example.

To run it you will need a Rust compiler installed: [get it here](https://www.rust-lang.org/tools/install)

Then clone the example project:

```bash
git clone ockam-network/kafka-example
```

And run the example:

```bash
cargo run --example kafka
```

Follow the instructions from the output to start another program and establish messaging between them.


### Rust project

We will use a simple Rust program to demonstrate the example.

To run it you will need a Rust compiler installed: [get it here](https://www.rust-lang.org/tools/install)

Set up a project:

```
cargo new --lib ockam_get_started

cd ockam_get_started

echo 'ockam = { version = "0", features = ["ockam_transport_tcp", "ockam_vault"] }' >> Cargo.toml

mkdir examples
```

Then create the example file:

```bash
touch examples/kakfa.rs
```

And put the following code there:

```rust
...
```

Now you can run the example:

```bash
cargo run --example kafka
```

Follow the instructions from the output to start another program and establish messaging between them.

### Docker image

We will use a simple Rust program to demonstrate the example.

We have it packaged as a docker image you can run as:

```bash
docker run ockam-network/kafka-example
```

Follow the instructions from the output to start another program and establish messaging between them.


## What just happened?

The example program is using Ockam framework to establish connection to a cloud node managed by Ockam Hub (../../rust/07-hub) and using the [Ockam Stream API](../../rust/xx-streams) to create topics in Confluent Cloud Kafka cluster.

Using these topics, the two programs then establish an [Ockam Secure Channel](../../rust/06-secure-channel) to exchange messages.

Every message you type is encrypted by Secure Channel, sent to the Hub Node and put in the Kafka topic by the Hub Node.

Finally it's fetched by the other program and decrypted by Secure Channel.

<img src="./kafka-end-to-end.png" width="100%">


The messages printed as `Push to the stream` and `Pull from the stream` are the messages sent to Kafka topics.

Messages in those logs are what the network or Kafka broker will see. As you can see the messages are encrypted and not exposed to the broker.

**NOTE** The encryption key is transient and generated on secure channel establishment. After you exit the example programs all the messages stored in Kafka will be lost as they can no longer be decrypted.


### Message flow

<img src="./sequence.png" width="100%">

## What's next?

- More about the [Ockam framework](../../rust/)
- More about [Secure Channels](../../rust/06-secure-channel)
- More about [Ockam Hub](../../rust/07-hub)
- More about [Ockam Streams and Kafka integration](../../rust/xx-streams)



Running the example:


```
MODE=responder ./ockam_kafka_e2ee

Created kafka topics.

Receiving messages from: generated_abc
Sending messages to: generated_dfe

Use the following arguments to run the other node:

IN=generated_dfe OUT=generated_abc MODE=initiator ./ockam_kafka_e2ee

Waiting for secure channel...

Secure channel established

Received message <encrypted blah>

Secure channel decrypted message: blah

```


```
IN=generated_dfe OUT=generated_abc MODE=initiator ./ockam_kafka_e2ee

Initiated stream
Established secure channel

Secure channel established

Enter your message:

>>> blah

Message sent through secure channel

Secure channel encrypted message: <encrypted blah>


```
