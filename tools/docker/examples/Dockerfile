FROM debian:10-slim
RUN apt-get -y update
RUN apt-get -y install libterm-readline-gnu-perl
RUN apt-get -y install apt-utils
RUN apt-get -y upgrade
RUN apt-get -y install clang lld curl git pkg-config libssl-dev
RUN curl https://sh.rustup.rs -sSf | sh -s -- -y
COPY tmpl/setup.sh /
COPY tmpl/get_started.sh /
COPY tmpl/website_guide.sh /
COPY tmpl/example_projects.sh /
COPY tmpl/entry.sh /
COPY tmpl/config.toml /.cargo
ADD tmpl/sccache-dist-v0.2.15-x86_64-unknown-linux-musl.tar.gz /scc
#ENTRYPOINT bash
ENTRYPOINT "/entry.sh"

