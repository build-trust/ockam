FROM ghcr.io/build-trust/ockam-builder@sha256:26c5bba9deca18953043a3a2368ec6bb00035676965432f35be24f88e316363a as executable

WORKDIR /app
COPY . /app
RUN cargo build --bin ockam --verbose --release

FROM gcr.io/distroless/cc@sha256:3ca297cd5426268b5ad21e3fbe5c568411e0dec49dbae8e2967d33207bc99773
COPY --from=executable /app/target/release/ockam /
ENTRYPOINT ["./ockam"]
