FROM ghcr.io/build-trust/ockam-builder@sha256:26c5bba9deca18953043a3a2368ec6bb00035676965432f35be24f88e316363a as builder
COPY . .
RUN set -xe; cd examples/rust/tcp_inlet_and_outlet; cargo build --release --examples

FROM ghcr.io/build-trust/ockam-base@sha256:40fcb081b6cf56d1e306d859d010a8a4c7b9a02e6b9bc468848c09653f714b74
COPY --from=builder /work/target/release/examples/* /usr/bin/
