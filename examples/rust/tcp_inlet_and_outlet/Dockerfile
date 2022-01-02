FROM ghcr.io/ockam-network/ockam/builder@sha256:e874d5c6323736e18c666aa26d15188b17f2342fee41bf20bdff463ace9bc4ae as builder
COPY . .
RUN set -xe; cd examples/rust/tcp_inlet_and_outlet; cargo build --release --examples

FROM ghcr.io/ockam-network/ockam/base@sha256:ea156477d425e92640ec8574663f598bc019269a12ed0fefb5ad48256afff4e0
COPY --from=builder /work/target/release/examples/* /usr/bin/
