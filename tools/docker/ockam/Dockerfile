FROM ghcr.io/build-trust/ockam-builder@sha256:d560cb1ed32124e68cb4dc5fc2902240090f122864715756ace06e99e38e6cb2 as executable

WORKDIR /app
COPY . /app
RUN cargo build --bin ockam --verbose --release

FROM gcr.io/distroless/cc@sha256:3ca297cd5426268b5ad21e3fbe5c568411e0dec49dbae8e2967d33207bc99773
COPY --from=executable /app/target/release/ockam /
ENTRYPOINT ["./ockam"]
