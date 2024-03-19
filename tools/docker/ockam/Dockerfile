FROM cgr.dev/chainguard/wolfi-base@sha256:9496b3fe80bcd98cf2af63b8cd904fce45554c6632093de0f5adf51a08c07d49 as executable

ENV PATH=/root/.cargo/bin:$PATH

WORKDIR /app
COPY . /app
RUN set -ex; \
    apk add rustup git glibc glibc-dev gcc; \
    rustup-init --no-update-default-toolchain -y && rustup show; \
    cargo build --bin ockam --verbose --release;

FROM cgr.dev/chainguard/cc-dynamic@sha256:9d5fbc697d0aaa054e6303efb93aa64f246bac257e98fa318d1141d9df378026
COPY --from=executable /app/target/release/ockam /
ENTRYPOINT ["./ockam"]
