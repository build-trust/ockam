FROM ghcr.io/build-trust/ockam-builder@sha256:3ea4c222da220c69b62a60d42d83769286e07bf8e41ef7cfdbeba18b7f22a488 AS builder

WORKDIR /build

COPY . ./

RUN cargo build --example ockam_kafka_bob
RUN cargo build --example ockam_kafka_alice

FROM ghcr.io/build-trust/ockam-base@sha256:40fcb081b6cf56d1e306d859d010a8a4c7b9a02e6b9bc468848c09653f714b74

COPY --from=builder /build/target/debug/examples/ockam_kafka_bob ./ockam_kafka_bob
COPY --from=builder /build/target/debug/examples/ockam_kafka_alice ./ockam_kafka_alice

ENV PATH="/work:${PATH}"
