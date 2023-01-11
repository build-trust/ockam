FROM ghcr.io/build-trust/ockam-builder@sha256:dddc009b53bb579ec3e2edf3353668c1d5213f444162bec884342dfc69450348 as executable

WORKDIR /app
COPY . /app
RUN cargo build --bin ockam --verbose --release

FROM gcr.io/distroless/cc@sha256:3ca297cd5426268b5ad21e3fbe5c568411e0dec49dbae8e2967d33207bc99773
COPY --from=executable /app/target/release/ockam /
ENTRYPOINT ["./ockam"]
