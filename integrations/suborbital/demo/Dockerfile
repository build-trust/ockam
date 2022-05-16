FROM ghcr.io/build-trust/ockam-builder@sha256:3ea4c222da220c69b62a60d42d83769286e07bf8e41ef7cfdbeba18b7f22a488 as builder
COPY . .
RUN set -xe; cd integrations/suborbital/demo; cargo build --release --bin ockam_tcp_outlet

# Note(thom): previously was ea156477d425e92640ec8574663f598bc019269a12ed0fefb5ad48256afff4e0, this is later, though.
FROM ghcr.io/build-trust/ockam-base@sha256:40fcb081b6cf56d1e306d859d010a8a4c7b9a02e6b9bc468848c09653f714b74
COPY --from=builder /work/target/release/ockam_tcp_outlet /usr/bin/

ENTRYPOINT ["/usr/bin/ockam_tcp_outlet"]
