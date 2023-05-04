FROM ghcr.io/build-trust/ockam-builder@sha256:cecb1860acd571278b2e7f8ecb3ffe405447ee844615134f93ddd11b1f3e2ca8 as executable

WORKDIR /app
COPY . /app
RUN cargo build --bin ockam --verbose --release

FROM gcr.io/distroless/cc@sha256:3ca297cd5426268b5ad21e3fbe5c568411e0dec49dbae8e2967d33207bc99773
COPY --from=executable /app/target/release/ockam /
ENTRYPOINT ["./ockam"]
