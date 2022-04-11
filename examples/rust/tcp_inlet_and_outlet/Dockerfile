FROM ghcr.io/ockam-network/ockam/builder@sha256:41df07cc61029f7293430acde1fccc8d7162cd3130aa32d151e440c9f9b31626 as builder
COPY . .
RUN set -xe; cd examples/rust/tcp_inlet_and_outlet; cargo build --release --examples

FROM ghcr.io/ockam-network/ockam/base@sha256:ea156477d425e92640ec8574663f598bc019269a12ed0fefb5ad48256afff4e0
COPY --from=builder /work/target/release/examples/* /usr/bin/
