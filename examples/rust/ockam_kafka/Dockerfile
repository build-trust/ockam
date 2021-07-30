FROM debian:10-slim
RUN apt-get -y update
RUN apt-get -y install libterm-readline-gnu-perl
RUN apt-get -y install apt-utils
RUN apt-get -y upgrade
RUN apt-get -y install clang lld curl git pkg-config libssl-dev
RUN curl https://sh.rustup.rs -sSf | sh -s -- -y

COPY . ./

RUN . ~/.cargo/env && cargo build

CMD . ~/.cargo/env && cargo run --example example



