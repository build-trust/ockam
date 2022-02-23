FROM ghcr.io/ockam-network/ockam/builder@sha256:a7d3dca4d7ec4385a12a81534524831030650aeb8dc4e820733e6bdb9e6bfd6a AS builder

WORKDIR /build

COPY . ./

RUN cargo build --example ockam_kafka_bob
RUN cargo build --example ockam_kafka_alice

FROM ghcr.io/ockam-network/ockam/base@sha256:ea156477d425e92640ec8574663f598bc019269a12ed0fefb5ad48256afff4e0

COPY --from=builder /build/target/debug/examples/ockam_kafka_bob ./ockam_kafka_bob
COPY --from=builder /build/target/debug/examples/ockam_kafka_alice ./ockam_kafka_alice

ENV PATH="/work:${PATH}"
