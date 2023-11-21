FROM cgr.dev/chainguard/wolfi-base as executable

ENV PATH=/root/.cargo/bin:$PATH

WORKDIR /app
COPY . /app
RUN set -ex; \
    apk add rustup git glibc glibc-dev gcc; \
    rustup-init --no-update-default-toolchain -y && rustup show; \
    cargo build --bin ockam --verbose --release;

FROM cgr.dev/chainguard/cc-dynamic
COPY --from=executable /app/target/release/ockam /
ENTRYPOINT ["./ockam"]
