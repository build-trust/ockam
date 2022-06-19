FROM ghcr.io/build-trust/ockam-builder:latest as executable

WORKDIR /app
COPY . /app
RUN cargo build --bin ockam --verbose --release

FROM gcr.io/distroless/cc
COPY --from=executable /app/target/release/ockam /
CMD ["./ockam"]
