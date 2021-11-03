FROM ghcr.io/ockam-network/ockam/builder_base:latest

ARG CMAKE_VERSION=3.19.2
ARG CMAKE_SHA256=4d8a6d852c530f263b22479aad196416bb4406447e918bd9759c6593b7f5f3f9
ARG RUSTUP_INIT_VERSION=1.24.3
ARG RUSTUP_INIT_SHA256=3dc5ef50861ee18657f9db2eeb7392f9c2a6c95c90ab41e45ab4ca71476b4338
ARG RUST_VERSION=1.56.1
ARG ERLANG_VERSION=24.1-1~debian~buster
ARG ERLANG_SHA256=079ca10987c51befeb5e7eb73c7ce4ba402763b4d74a49fd0b871ff18019f51b
ARG ELIXIR_VERSION=1.12.2-1~debian~buster
ARG ELIXIR_SHA256=5e8251c5d2557373ecfab986fa481844a2f659597abbfb623f45ad3a1974bb1f

ENV CMAKE_HOME=/opt/cmake \
    RUSTUP_HOME=/opt/rust/rustup \
    CARGO_HOME=/usr/rust/cargo \
    JAVA_HOME=/opt/java/openjdk

COPY emulator.sh /opt/empulators/x86_64-linux-gnu.sh
COPY toolchain.cmake /opt/toolchains/x86_64-linux-gnu.cmake

RUN set -xe; \
    ln -s /opt/toolchains/x86_64-linux-gnu.cmake /opt/toolchains/detected_target.cmake; \
# Setup cmake
    CMAKE_PACKAGE="cmake-${CMAKE_VERSION}-Linux-x86_64.tar.gz"; \
    curl -sSOL "https://github.com/Kitware/CMake/releases/download/v${CMAKE_VERSION}/${CMAKE_PACKAGE}"; \
    echo "${CMAKE_SHA256}  ${CMAKE_PACKAGE}" | sha256sum -c -; \
    tar -xf "${CMAKE_PACKAGE}" -C /opt/; \
    mv "/opt/cmake-${CMAKE_VERSION}-Linux-x86_64" "${CMAKE_HOME}"; \
    rm -rf "${CMAKE_PACKAGE}"; \
# Setup rust
    apt-get update; \
    DEBIAN_FRONTEND=noninteractive apt-get install --assume-yes --no-install-recommends gcc gcc-multilib libssl-dev pkg-config procps qemu-system-arm; \
    curl -sSOL \
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
    curl -sSOL "https://packages.erlang-solutions.com/erlang/debian/pool/${ERLANG_PACKAGE}"; \
    echo "${ERLANG_SHA256}  ${ERLANG_PACKAGE}" | sha256sum -c -; \
    dpkg -i "${ERLANG_PACKAGE}"; \
    rm -rf "${ERLANG_PACKAGE}"; \
# Setup elixir
    ELIXIR_PACKAGE="elixir_${ELIXIR_VERSION}_all.deb"; \
    curl -sSOL "https://packages.erlang-solutions.com/erlang/debian/pool/${ELIXIR_PACKAGE}"; \
    echo "${ELIXIR_SHA256}  ${ELIXIR_PACKAGE}" | sha256sum -c; \
    dpkg -i "${ELIXIR_PACKAGE}"; \
    rm -rf "${ELIXIR_PACKAGE}"; \
    mix local.hex --force && mix local.rebar --force; \
# Setup jdk
    cd /tmp; \
    JAVA_PACKAGE='OpenJDK11U-jdk_x64_linux_hotspot_11.0.13_8.tar.gz'; \
    curl -sSOL \
      "https://github.com/adoptium/temurin11-binaries/releases/download/jdk-11.0.13%2B8/${JAVA_PACKAGE}"; \
    echo "3b1c0c34be4c894e64135a454f2d5aaa4bd10aea04ec2fa0c0efe6bb26528e30  ${JAVA_PACKAGE}"; \
    mv "${JAVA_PACKAGE}" openjdk.tar.gz; \
    mkdir -p "${JAVA_HOME}"; \
    cd "${JAVA_HOME}"; \
    tar -xf /tmp/openjdk.tar.gz --strip-components=1; \
    rm -rf /tmp/openjdk.tar.gz;

ENV PATH="${JAVA_HOME}/bin:${RUSTUP_HOME}/bin:${CARGO_HOME}/bin:${CMAKE_HOME}/bin:$PATH" \
    AR=/usr/bin/ar \
    AS=/usr/bin/as \
    CC=/usr/local/bin/gcc \
    CPP=/usr/local/bin/cpp \
    CXX=/usr/local/bin/g++
