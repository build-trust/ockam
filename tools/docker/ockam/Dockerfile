FROM ghcr.io/build-trust/ockam-builder@sha256:60e2c1e45d54e31b50f3e56fe202276ecc4812ccac88c84c9af8910cf787a81d as executable

WORKDIR /app
COPY . /app
RUN cargo build --bin ockam --verbose --release

FROM gcr.io/distroless/cc@sha256:3ca297cd5426268b5ad21e3fbe5c568411e0dec49dbae8e2967d33207bc99773
COPY --from=executable /app/target/release/ockam /
ENTRYPOINT ["./ockam"]
