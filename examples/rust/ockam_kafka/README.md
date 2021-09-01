# Example: Secure Channels through Apache Kafka

This project contains an example code to create Secure Channels via Ockam Streams using Apache Kafka as a storage.

This project is used to build the docker image for the [end-to-end encryption with Kafka guide](../../../documentation/use-cases/
end-to-end-encryption-through-kafka)

## Cargo build

To build the project you will need rust installed.

If you don't have it, please [install](https://www.rust-lang.org/tools/install) the latest version of Rust:

```
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
````

Then you can run Bob and Alice examples described in the guide by running:

```bash
cargo run --example ockam_kafka_bob
```

```bash
cargo run --example ockam_kafka_alice
```

## Docker build

You can build the docker image used in the example yourself by running:

```bash
docker build -t ockam_kafka .
```

Then you can run Bob and Alice examples as:

```bash
docker run ockam_kafka ockam_kafka_bob
```

```bash
docker run ockam_kafka ockam_kafka_alice
```

