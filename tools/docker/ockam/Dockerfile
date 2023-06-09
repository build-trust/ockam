FROM ghcr.io/build-trust/ockam-builder@sha256:5ab42598e35509cad3ea9c1e1bd0ed135ed1340c6ae44b762b1c8bbec31d5c68 as executable

WORKDIR /app
COPY . /app
RUN cargo build --bin ockam --verbose --release

FROM gcr.io/distroless/cc@sha256:3ca297cd5426268b5ad21e3fbe5c568411e0dec49dbae8e2967d33207bc99773
COPY --from=executable /app/target/release/ockam /
ENTRYPOINT ["./ockam"]
