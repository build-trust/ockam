FROM ghcr.io/ockam-network/ockam/builder AS builder

WORKDIR /build

COPY . ./

RUN cargo build --example bob
RUN cargo build --example alice

FROM ghcr.io/ockam-network/ockam/base

COPY --from=builder /build/target/debug/examples/bob ./bob
COPY --from=builder /build/target/debug/examples/alice ./alice

ENV PATH="/work:${PATH}"
