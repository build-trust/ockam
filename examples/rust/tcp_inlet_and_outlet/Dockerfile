FROM ghcr.io/build-trust/ockam-builder@sha256:cecb1860acd571278b2e7f8ecb3ffe405447ee844615134f93ddd11b1f3e2ca8 as builder
COPY . .
RUN set -xe; cd examples/rust/tcp_inlet_and_outlet; cargo build --release --examples

FROM ghcr.io/build-trust/ockam-base@sha256:40fcb081b6cf56d1e306d859d010a8a4c7b9a02e6b9bc468848c09653f714b74
COPY --from=builder /work/target/release/examples/* /usr/bin/
