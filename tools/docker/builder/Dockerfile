ARG BASE_IMAGE=ghcr.io/build-trust/ockam-builder-base:latest
FROM ${BASE_IMAGE}

ARG CMAKE_VERSION=3.22.1
ARG CMAKE_SHA256=73565c72355c6652e9db149249af36bcab44d9d478c5546fd926e69ad6b43640
ARG RUSTUP_INIT_VERSION=1.24.3
ARG RUSTUP_INIT_SHA256=3dc5ef50861ee18657f9db2eeb7392f9c2a6c95c90ab41e45ab4ca71476b4338
ARG RUST_VERSION=1.60.0
ARG ERLANG_VERSION=24.1.7-1~debian~buster
ARG ERLANG_SHA256=89c98e177f70593a9f64f3c9962393a51e6730d16071820600d19d4aaa732412
ARG ELIXIR_VERSION=1.12.2-1~debian~buster
ARG ELIXIR_SHA256=5e8251c5d2557373ecfab986fa481844a2f659597abbfb623f45ad3a1974bb1f
ARG NODEJS_VERSION=16.13.1
ARG NODEJS_SHA256=a3721f87cecc0b52b0be8587c20776ac7305db413751db02c55aa2bffac15198

ENV CMAKE_HOME=/opt/cmake \
    RUSTUP_HOME=/opt/rust/rustup \
    CARGO_HOME=/usr/rust/cargo \
    JAVA_HOME=/opt/java/openjdk \
    NODEJS_HOME=/opt/nodejs

COPY emulator.sh /opt/empulators/x86_64-linux-gnu.sh
COPY toolchain.cmake /opt/toolchains/x86_64-linux-gnu.cmake

RUN set -xe; \
    ln -s /opt/toolchains/x86_64-linux-gnu.cmake /opt/toolchains/detected_target.cmake; \
# Setup cmake
    CMAKE_PACKAGE="cmake-${CMAKE_VERSION}-linux-x86_64.tar.gz"; \
    curl --proto '=https' --tlsv1.2 -sSOL \
      "https://github.com/Kitware/CMake/releases/download/v${CMAKE_VERSION}/${CMAKE_PACKAGE}"; \
    echo "${CMAKE_SHA256}  ${CMAKE_PACKAGE}" | sha256sum -c -; \
    tar -xf "${CMAKE_PACKAGE}" -C /opt/; \
    mv "/opt/cmake-${CMAKE_VERSION}-linux-x86_64" "${CMAKE_HOME}"; \
    rm -rf "${CMAKE_PACKAGE}"; \
# Setup nodejs
    NODEJS_PACKAGE="node-v${NODEJS_VERSION}-linux-x64.tar.xz"; \
    curl --proto '=https' --tlsv1.2 -sSOL \
      "https://nodejs.org/dist/v${NODEJS_VERSION}/node-v${NODEJS_VERSION}-linux-x64.tar.xz"; \
    echo "${NODEJS_SHA256}  ${NODEJS_PACKAGE}" | sha256sum -c -; \
    tar -xf "${NODEJS_PACKAGE}" -C /opt/; \
    mv "/opt/node-v${NODEJS_VERSION}-linux-x64" "${NODEJS_HOME}"; \
    rm -rf "${NODEJS_PACKAGE}"; \
# Setup rust
    apt-get update; \
    DEBIAN_FRONTEND=noninteractive apt-get install --assume-yes --no-install-recommends \
      gcc gcc-multilib libssl-dev pkg-config procps qemu-system-arm libdbus-1-dev; \
    curl --proto '=https' --tlsv1.2 -sSOL \
      "https://static.rust-lang.org/rustup/archive/${RUSTUP_INIT_VERSION}/x86_64-unknown-linux-gnu/rustup-init"; \
    echo "${RUSTUP_INIT_SHA256}  rustup-init" | sha256sum -c -; \
    chmod +x rustup-init; \
    ./rustup-init -y --no-modify-path --profile minimal \
      --default-toolchain "$RUST_VERSION" --default-host x86_64-unknown-linux-gnu; \
    rm rustup-init; \
    chmod -R a+w "$RUSTUP_HOME" "$CARGO_HOME"; \
    PATH="${RUSTUP_HOME}/bin:${CARGO_HOME}/bin:$PATH"; \
    rustup component add rustfmt; \
    rustup install nightly; \
    rustup component add clippy; \
    cargo install --locked cargo-deny; \
    chmod -R a+w "$RUSTUP_HOME" "$CARGO_HOME"; \
# Setup erlang
    apt-get update; \
    DEBIAN_FRONTEND=noninteractive apt-get install --assume-yes --no-install-recommends \
      libncurses5 libwxbase3.0-0v5 libwxgtk3.0-gtk3-0v5 libsctp1; \
    apt-get clean; \
    rm -rf /var/lib/apt/lists/*; \
    ERLANG_PACKAGE="esl-erlang_${ERLANG_VERSION}_amd64.deb"; \
    curl --proto '=https' --tlsv1.2 -sSOL "https://packages.erlang-solutions.com/erlang/debian/pool/${ERLANG_PACKAGE}"; \
    echo "${ERLANG_SHA256}  ${ERLANG_PACKAGE}" | sha256sum -c -; \
    dpkg -i "${ERLANG_PACKAGE}"; \
    rm -rf "${ERLANG_PACKAGE}"; \
# Setup elixir
    ELIXIR_PACKAGE="elixir_${ELIXIR_VERSION}_all.deb"; \
    curl --proto '=https' --tlsv1.2 -sSOL "https://packages.erlang-solutions.com/erlang/debian/pool/${ELIXIR_PACKAGE}"; \
    echo "${ELIXIR_SHA256}  ${ELIXIR_PACKAGE}" | sha256sum -c; \
    dpkg -i "${ELIXIR_PACKAGE}"; \
    rm -rf "${ELIXIR_PACKAGE}"; \
    mix local.hex --force && mix local.rebar --force; \
# Setup jdk
    cd /tmp; \
    JAVA_PACKAGE='OpenJDK17-jdk_x64_linux_hotspot_17_35.tar.gz'; \
    curl --proto '=https' --tlsv1.2 -sSOL \
      "https://github.com/adoptium/temurin17-binaries/releases/download/jdk-17%2B35/${JAVA_PACKAGE}"; \
    echo "6f1335d9a7855159f982dac557420397be9aa85f3f7bc84e111d25871c02c0c7  ${JAVA_PACKAGE}" | sha256sum -c; \
    mv "${JAVA_PACKAGE}" openjdk.tar.gz; \
    mkdir -p "${JAVA_HOME}"; \
    cd "${JAVA_HOME}"; \
    tar -xf /tmp/openjdk.tar.gz --strip-components=1; \
    rm -rf /tmp/openjdk.tar.gz; \
# Setup jq
    cd /tmp; \
    curl --proto '=https' --tlsv1.2 -sSOL "https://github.com/stedolan/jq/releases/download/jq-1.6/jq-linux64"; \
    echo "af986793a515d500ab2d35f8d2aecd656e764504b789b66d7e1a0b727a124c44  jq-linux64" | sha256sum -c; \
    mv jq-linux64 /usr/bin/jq; \
    chmod +x /usr/bin/jq

ENV PATH="${JAVA_HOME}/bin:${RUSTUP_HOME}/bin:${CARGO_HOME}/bin:${CMAKE_HOME}/bin:${NODEJS_HOME}/bin:$PATH" \
    AR=/usr/bin/ar \
    AS=/usr/bin/as \
    CC=/usr/local/bin/gcc \
    CPP=/usr/local/bin/cpp \
    CXX=/usr/local/bin/g++
