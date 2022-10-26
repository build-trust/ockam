FROM ghcr.io/build-trust/ockam-builder@sha256:9598b7bc30bb23e9e64cd95cb96a30af9695e6a16ce1834e6c17c3c62bc81c2e as builder
COPY . .
RUN set -xe; cd examples/rust/tcp_inlet_and_outlet; cargo build --release --examples

FROM ghcr.io/build-trust/ockam-base@sha256:40fcb081b6cf56d1e306d859d010a8a4c7b9a02e6b9bc468848c09653f714b74
COPY --from=builder /work/target/release/examples/* /usr/bin/
