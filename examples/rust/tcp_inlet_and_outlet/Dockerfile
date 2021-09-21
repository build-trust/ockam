FROM ghcr.io/ockam-network/ockam/builder as builder
COPY . .
RUN set -xe; cd examples/rust/tcp_inlet_and_outlet; cargo build --release --examples

FROM ghcr.io/ockam-network/ockam/base
COPY --from=builder /work/examples/rust/tcp_inlet_and_outlet/target/release/examples/* /usr/bin/
