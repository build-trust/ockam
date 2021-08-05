FROM ghcr.io/ockam-network/ockam/builder_base:latest

ARG CMAKE_VERSION=3.19.2
ARG CMAKE_SHA256=4d8a6d852c530f263b22479aad196416bb4406447e918bd9759c6593b7f5f3f9
ARG RUSTUP_INIT_VERSION=1.23.1
ARG RUSTUP_INIT_SHA256=ed7773edaf1d289656bdec2aacad12413b38ad0193fff54b2231f5140a4b07c5
ARG RUST_VERSION=1.51.0
ARG ERLANG_VERSION=23.2.1-1~debian~buster
ARG ERLANG_SHA256=b6fba5aa2f9c3c51d3f0373713663d7cbd1df5109e4d0d69e96c99abc536e637
ARG ELIXIR_VERSION=1.11.2-1~debian~buster
ARG ELIXIR_SHA256=01fa394c470f675c1b093f313d40adbf055a58877461a922634d94d6e9b48d29

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
    DEBIAN_FRONTEND=noninteractive apt-get install --assume-yes --no-install-recommends gcc gcc-multilib libssl-dev pkg-config procps; \
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
    rustup component add clippy; \
    cargo install cargo-audit; \
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
    JAVA_PACKAGE='OpenJDK11U-jdk_x64_linux_hotspot_2020-10-23-08-51.tar.gz'; \
    curl -sSOL \
      "https://github.com/AdoptOpenJDK/openjdk11-binaries/releases/download/jdk11u-2020-10-23-08-51/${JAVA_PACKAGE}"; \
    echo "a118a5d0d6a288d05bbe99bb86e9a8d70ce40c67ba0ebc917f23250705fd3644  ${JAVA_PACKAGE}"; \
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
