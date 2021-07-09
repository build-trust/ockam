FROM debian:10-slim AS builder
RUN apt-get -y update
RUN apt-get -y install libterm-readline-gnu-perl
RUN apt-get -y install apt-utils
RUN apt-get -y upgrade
RUN apt-get -y install clang lld curl git pkg-config libssl-dev
RUN curl https://sh.rustup.rs -sSf | sh -s -- -y

WORKDIR /build

COPY . ./

RUN . ~/.cargo/env && cargo build --example bob
RUN . ~/.cargo/env && cargo build --example alice

FROM debian:10-slim

WORKDIR /run

COPY --from=builder /build/target/debug/examples/bob ./bob
COPY --from=builder /build/target/debug/examples/alice ./alice

ENV PATH="/run:${PATH}"

