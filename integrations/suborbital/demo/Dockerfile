FROM ghcr.io/build-trust/ockam-builder@sha256:1930541843c0533f5f862432e2f293590f1832de3210b7c1ef27a1e9a3979650 as builder
COPY . .
RUN set -xe; cd integrations/suborbital/demo; cargo build --release --bin ockam_tcp_outlet

# Note(thom): previously was ea156477d425e92640ec8574663f598bc019269a12ed0fefb5ad48256afff4e0, this is later, though.
FROM ghcr.io/build-trust/ockam-base@sha256:40fcb081b6cf56d1e306d859d010a8a4c7b9a02e6b9bc468848c09653f714b74
COPY --from=builder /work/target/release/ockam_tcp_outlet /usr/bin/

ENTRYPOINT ["/usr/bin/ockam_tcp_outlet"]
