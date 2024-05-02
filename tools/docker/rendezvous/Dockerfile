FROM messense/cargo-zigbuild as binary

WORKDIR /work
COPY . /work
RUN set -ex; \
    rustup target add x86_64-unknown-linux-musl; \
    cargo zigbuild --target x86_64-unknown-linux-musl --release --example rendezvous_server;

FROM cgr.dev/chainguard/wolfi-base
COPY --from=binary /work/target/x86_64-unknown-linux-musl/release/examples/rendezvous_server /opt/rendezvous_server

EXPOSE 4000

CMD ["/opt/rendezvous_server"]
