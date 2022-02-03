FROM ghcr.io/ockam-network/ockam/builder@sha256:ea810e06304a0178a8716060033524ff92dfce2d51c9b6dbc17305d11e967851 as builder
COPY . .
RUN set -xe; cd examples/rust/tcp_inlet_and_outlet; cargo build --release --examples

FROM ghcr.io/ockam-network/ockam/base@sha256:ea156477d425e92640ec8574663f598bc019269a12ed0fefb5ad48256afff4e0
COPY --from=builder /work/target/release/examples/* /usr/bin/
