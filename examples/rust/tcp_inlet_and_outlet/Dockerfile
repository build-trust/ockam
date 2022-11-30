FROM ghcr.io/build-trust/ockam-builder@sha256:60e2c1e45d54e31b50f3e56fe202276ecc4812ccac88c84c9af8910cf787a81d as builder
COPY . .
RUN set -xe; cd examples/rust/tcp_inlet_and_outlet; cargo build --release --examples

FROM ghcr.io/build-trust/ockam-base@sha256:40fcb081b6cf56d1e306d859d010a8a4c7b9a02e6b9bc468848c09653f714b74
COPY --from=builder /work/target/release/examples/* /usr/bin/
