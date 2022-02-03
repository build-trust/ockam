FROM ghcr.io/ockam-network/ockam/builder@sha256:ea810e06304a0178a8716060033524ff92dfce2d51c9b6dbc17305d11e967851 AS builder

WORKDIR /build

COPY . ./

RUN cargo build --example ockam_kafka_bob
RUN cargo build --example ockam_kafka_alice

FROM ghcr.io/ockam-network/ockam/base@sha256:ea156477d425e92640ec8574663f598bc019269a12ed0fefb5ad48256afff4e0

COPY --from=builder /build/target/debug/examples/ockam_kafka_bob ./ockam_kafka_bob
COPY --from=builder /build/target/debug/examples/ockam_kafka_alice ./ockam_kafka_alice

ENV PATH="/work:${PATH}"
