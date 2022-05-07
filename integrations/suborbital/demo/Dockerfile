FROM ghcr.io/build-trust/ockam/builder@sha256:41df07cc61029f7293430acde1fccc8d7162cd3130aa32d151e440c9f9b31626 as builder
COPY . .
RUN set -xe; cd integrations/suborbital/demo; cargo build --release --bin ockam_tcp_outlet

# Note(thom): previously was ea156477d425e92640ec8574663f598bc019269a12ed0fefb5ad48256afff4e0, this is later, though.
FROM ghcr.io/build-trust/ockam/base@sha256:cf180692c3d5ab50b42d3a63067b7b2f8445d2dc0fd365913c2224450ee36a65
COPY --from=builder /work/target/release/ockam_tcp_outlet /usr/bin/

ENTRYPOINT ["/usr/bin/ockam_tcp_outlet"]
