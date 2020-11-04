FROM buildpack-deps:buster-scm

# install base Ockam build system dependencies
RUN wget https://packages.erlang-solutions.com/erlang-solutions_2.0_all.deb 
RUN echo 'a191b37f11d8133c12ae146d76072f5f1d884d80e1f1bccd8c7aeb55f70a72cb  erlang-solutions_2.0_all.deb' | sha256sum -c
RUN dpkg -i erlang-solutions_2.0_all.deb
RUN apt update && apt install -y cmake default-jdk build-essential gcc g++ esl-erlang pkg-config libssl-dev
RUN cmake --version
RUN javac --version

# install rustup and rust toolchains
# based on https://github.com/rust-lang/docker-rust/blob/a5896ce68cee87e80a44d078afbf05d5b679cdbc/1.47.0/buster/Dockerfile
ENV RUSTUP_HOME=/usr/local/rustup \
    CARGO_HOME=/usr/local/cargo \
    PATH=/usr/local/cargo/bin:$PATH \
    RUST_VERSION=1.47.0

RUN set -eux; \
    dpkgArch="$(dpkg --print-architecture)"; \
    case "${dpkgArch##*-}" in \
        amd64) rustArch='x86_64-unknown-linux-gnu'; rustupSha256='49c96f3f74be82f4752b8bffcf81961dea5e6e94ce1ccba94435f12e871c3bdb' ;; \
        armhf) rustArch='armv7-unknown-linux-gnueabihf'; rustupSha256='5a2be2919319e8778698fa9998002d1ec720efe7cb4f6ee4affb006b5e73f1be' ;; \
        arm64) rustArch='aarch64-unknown-linux-gnu'; rustupSha256='d93ef6f91dab8299f46eef26a56c2d97c66271cea60bf004f2f088a86a697078' ;; \
        i386) rustArch='i686-unknown-linux-gnu'; rustupSha256='e3d0ae3cfce5c6941f74fed61ca83e53d4cd2deb431b906cbd0687f246efede4' ;; \
        *) echo >&2 "unsupported architecture: ${dpkgArch}"; exit 1 ;; \
    esac; \
    url="https://static.rust-lang.org/rustup/archive/1.22.1/${rustArch}/rustup-init"; \
    wget "$url"; \
    echo "${rustupSha256} *rustup-init" | sha256sum -c -; \
    chmod +x rustup-init; \
    ./rustup-init -y --no-modify-path --profile minimal --default-toolchain $RUST_VERSION --default-host ${rustArch}; \
    rm rustup-init; \
    chmod -R a+w $RUSTUP_HOME $CARGO_HOME; \
    rustup --version; \
    cargo --version; \
    rustc --version;

# install Elixir and mix tools
RUN wget https://packages.erlang-solutions.com/erlang/debian/pool/elixir_1.10.4-1~debian~buster_all.deb
RUN echo '07f2ac5980f856353e2c7d3f04f16176c9ad8967ccfa39a6e07cf676c704807f  elixir_1.10.4-1~debian~buster_all.deb' | sha256sum -c
RUN dpkg -i 'elixir_1.10.4-1~debian~buster_all.deb'
RUN mix local.hex --force && mix local.rebar --force
RUN elixir --version

# TODO: only copy implementation language dependency lock files for pre-building
COPY . .
# append "|| true" so any successful build step persists and the image is created
RUN ./gradlew build || true
