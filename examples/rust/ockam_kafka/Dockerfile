FROM ghcr.io/ockam-network/ockam/builder AS builder

WORKDIR /build

COPY . ./

RUN cargo build --example ockam_kafka_bob
RUN cargo build --example ockam_kafka_alice

FROM ghcr.io/ockam-network/ockam/base

COPY --from=builder /build/target/debug/examples/ockam_kafka_bob ./ockam_kafka_bob
COPY --from=builder /build/target/debug/examples/ockam_kafka_alice ./ockam_kafka_alice

ENV PATH="/work:${PATH}"
