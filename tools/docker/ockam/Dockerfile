FROM ghcr.io/build-trust/ockam-builder@sha256:84bda332c6f87eecaf5418f403053cb868eb007d7931de557f1f692a8a95cc8b as executable

WORKDIR /app
COPY . /app
RUN cargo build --bin ockam --verbose --release

FROM gcr.io/distroless/cc@sha256:3ca297cd5426268b5ad21e3fbe5c568411e0dec49dbae8e2967d33207bc99773
COPY --from=executable /app/target/release/ockam /
ENTRYPOINT ["./ockam"]
