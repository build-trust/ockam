FROM gcc:11.2.0@sha256:0eaeaee298c154ecd383cabe5301a5df9fde0cd3a729124caf574500b936389b

ARG RUSTUP_INIT_VERSION=1.24.3
ARG RUST_VERSION=1.61.0
ARG ERLANG_VERSION=24.1.7-1~debian~buster
ARG ELIXIR_VERSION=1.12.2-1~debian~buster
ARG NODEJS_VERSION=16.13.1
ARG COSIGN_VERSION=1.9.0
ARG MOLD_VERSION=1.3.1

ARG ELIXIR_SHA256=5e8251c5d2557373ecfab986fa481844a2f659597abbfb623f45ad3a1974bb1f

ENV RUSTUP_HOME=/opt/rust/rustup \
    CARGO_HOME=/usr/rust/cargo \
    JAVA_HOME=/opt/java/openjdk \
    NODEJS_HOME=/opt/nodejs \
    MOLD_HOME=/opt/mold

RUN set -xe; \
    case $(dpkg --print-architecture) in \
      "amd64") \
        echo "Using AMD64"; \
        COSIGN_SHA256="47e2596a38e619b72e736fd171eeeaadaf6bd22d6e87a767b339168a87b62761" && COSIGN_OS="amd64"; \
        RUSTUP_INIT_SHA256="3dc5ef50861ee18657f9db2eeb7392f9c2a6c95c90ab41e45ab4ca71476b4338"; \
        NODEJS_SHA256="a3721f87cecc0b52b0be8587c20776ac7305db413751db02c55aa2bffac15198" && NODEJS_OS="linux-x64"; \
        ERLANG_SHA256="89c98e177f70593a9f64f3c9962393a51e6730d16071820600d19d4aaa732412" && ERLANG_OS="amd64"; \
        JDK_SHA256="81f5bed21077f9fbb04909b50391620c78b9a3c376593c0992934719c0de6b73" && JDK_OS="x64"; \
        JQ_SHA256="af986793a515d500ab2d35f8d2aecd656e764504b789b66d7e1a0b727a124c44" && JQ_OS="linux64"; \
        MOLD_SHA256="3893f89e5e0dcddcecc9f2ee17f14ad94fbf8b324eca45974b965353a50dac37" && MOLD_OS="x86_64-linux"; \
        OS="x86_64-unknown-linux-gnu"; \
      ;; \
      "arm64") \
        export CARGO_NET_GIT_FETCH_WITH_CLI=true; \
        echo "Using ARM64"; \
        COSIGN_SHA256="abd7ec116dd7e7980f08e67d2c7478ae1cdc97adf778aff76d8a737a908670d8" && COSIGN_OS="arm64"; \
        RUSTUP_INIT_SHA256="32a1532f7cef072a667bac53f1a5542c99666c4071af0c9549795bbdb2069ec1"; \
        NODEJS_SHA256="af1127594d6dae96d3f1d307174daa5084d9c9027eb6fc02548022257f4b0a6a" && NODEJS_OS="linux-arm64"; \
        ERLANG_SHA256="80e95df44cba03f9abefa54ab1452d97b9969a6d164792e827e59a1116377bbd" && ERLANG_OS="arm64"; \
        JDK_SHA256="2e3c19c1707205c6b90cc04b416e8d83078ed98417d5a69dce3cf7dc0d7cfbca" && JDK_OS="aarch64"; \
        JQ_SHA256="af986793a515d500ab2d35f8d2aecd656e764504b789b66d7e1a0b727a124c44" && JQ_OS="linux64"; \
        MOLD_SHA256="24ea75ada4337c5f509ad2eecf1a82d6047ab58c415a78374dfa7e6d69b06737" && MOLD_OS="aarch64-linux"; \
        OS="aarch64-unknown-linux-gnu"; \
      ;; \
      *) \
        echo "unknown arch:" \
        uname -a; \
        exit 1; \
      ;; \
    esac; \
# Setup base tools
    apt-get update; \
    DEBIAN_FRONTEND=noninteractive apt-get install --assume-yes --no-install-recommends ca-certificates curl locales xz-utils clang; \
# Setup locale
    LANG=en_US.UTF-8; \
    echo $LANG UTF-8 > /etc/locale.gen; \
    locale-gen; \
    update-locale LANG=$LANG; \
# Setup Bats
    mkdir /usr/lib/bats && cd /usr/lib/bats && git init; \
    git submodule add https://github.com/bats-core/bats-core.git; \
    cd bats-core && ./install.sh /usr; \
    cd - && rm -rf bats-core; \
    git submodule add https://github.com/bats-core/bats-support.git; \
    git submodule add https://github.com/bats-core/bats-assert.git; \
# Setup nodejs
    NODEJS_PACKAGE="node-v${NODEJS_VERSION}-${NODEJS_OS}.tar.xz"; \
    curl --proto '=https' --tlsv1.2 -sSOL \
      "https://nodejs.org/dist/v${NODEJS_VERSION}/node-v${NODEJS_VERSION}-${NODEJS_OS}.tar.xz"; \
    echo "${NODEJS_SHA256}  ${NODEJS_PACKAGE}" | sha256sum -c -; \
    tar -xf "${NODEJS_PACKAGE}" -C /opt/; \
    mv "/opt/node-v${NODEJS_VERSION}-${NODEJS_OS}" "${NODEJS_HOME}"; \
    rm -rf "${NODEJS_PACKAGE}"; \
# Setup rust
    apt-get update; \
    DEBIAN_FRONTEND=noninteractive apt-get install --assume-yes \
      libssl-dev pkg-config procps qemu-system-arm libdbus-1-dev cmake; \
    curl --proto '=https' --tlsv1.2 -sSOL \
      "https://static.rust-lang.org/rustup/archive/${RUSTUP_INIT_VERSION}/${OS}/rustup-init"; \
    echo "${RUSTUP_INIT_SHA256}  rustup-init" | sha256sum -c -; \
    chmod +x rustup-init; \
    ./rustup-init -y --no-modify-path --profile minimal \
      --default-toolchain "$RUST_VERSION" --default-host ${OS}; \
    rm rustup-init; \
    chmod -R a+w "$RUSTUP_HOME" "$CARGO_HOME"; \
    PATH="${RUSTUP_HOME}/bin:${CARGO_HOME}/bin:$PATH"; \
    rustup component add rustfmt; \
    rustup install nightly; \
    rustup component add clippy; \
    cargo install --locked cargo-deny; \
    cargo install --locked cargo-nextest; \
    chmod -R a+w "$RUSTUP_HOME" "$CARGO_HOME"; \
# Setup erlang
    apt-get update; \
    DEBIAN_FRONTEND=noninteractive apt-get install --assume-yes --no-install-recommends \
      libncurses5 libwxbase3.0-0v5 libwxgtk3.0-gtk3-0v5 libsctp1; \
    apt-get clean; \
    rm -rf /var/lib/apt/lists/*; \
    ERLANG_PACKAGE="esl-erlang_${ERLANG_VERSION}_${ERLANG_OS}.deb"; \
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
    echo "${JDK_OS}"; \
    JAVA_PACKAGE="OpenJDK17U-jdk_${JDK_OS}_linux_hotspot_17.0.3_7.tar.gz"; \
    curl --proto '=https' --tlsv1.2 -sSOL \
      "https://github.com/adoptium/temurin17-binaries/releases/download/jdk-17.0.3%2B7/${JAVA_PACKAGE}"; \
    echo "${JDK_SHA256}  ${JAVA_PACKAGE}" | sha256sum -c; \
    mv "${JAVA_PACKAGE}" openjdk.tar.gz; \
    mkdir -p "${JAVA_HOME}"; \
    cd "${JAVA_HOME}"; \
    tar -xf /tmp/openjdk.tar.gz --strip-components=1; \
    rm -rf /tmp/openjdk.tar.gz; \
# Setup jq
    cd /tmp; \
    curl --proto '=https' --tlsv1.2 -sSOL "https://github.com/stedolan/jq/releases/download/jq-1.6/jq-${JQ_OS}"; \
    echo "${JQ_SHA256}  jq-${JQ_OS}" | sha256sum -c; \
    mv jq-${JQ_OS} /usr/bin/jq; \
    chmod +x /usr/bin/jq; \
# Setup cosign
    cd /tmp; \
    COSIGN_PACKAGE="cosign-linux-${COSIGN_OS}"; \
    curl --proto '=https' --tlsv1.2 -sSOL "https://github.com/sigstore/cosign/releases/download/v${COSIGN_VERSION}/${COSIGN_PACKAGE}"; \
    echo "${COSIGN_SHA256}  ${COSIGN_PACKAGE}" | sha256sum -c; \
    mv ${COSIGN_PACKAGE} /usr/bin/cosign; \
    chmod +x /usr/bin/cosign; \
# Setup mold linker
    apt-get update; \
    DEBIAN_FRONTEND=noninteractive apt-get install --assume-yes --no-install-recommends clang; \
    cd /tmp; \
    MOLD_PACKAGE="mold-${MOLD_VERSION}-${MOLD_OS}.tar.gz"; \
    curl --proto '=https' --tlsv1.2 -sSOL "https://github.com/rui314/mold/releases/download/v${MOLD_VERSION}/${MOLD_PACKAGE}"; \
    echo "${MOLD_SHA256}  ${MOLD_PACKAGE}" | sha256sum -c; \
    mkdir -p "${MOLD_HOME}"; \
    cd "${MOLD_HOME}"; \
    tar -xf /tmp/${MOLD_PACKAGE} --strip-components=1; \
    rm -rf /tmp/${MOLD_PACKAGE}; \
    chown -Rh root:root ${MOLD_HOME}/*; \
# Cleanup
    apt-get clean; \
    rm -rf /var/lib/apt/lists/*

ENV PATH="${CARGO_HOME}/bin:${CMAKE_HOME}/bin:${NODEJS_HOME}/bin:${MOLD_HOME}/bin:$PATH" \
    AR=/usr/bin/ar \
    AS=/usr/bin/as \
    CC=/usr/local/bin/gcc \
    CPP=/usr/local/bin/cpp \
    CXX=/usr/local/bin/g++ \
    LANG=en_US.UTF-8 \
    BATS_LIB=/usr/lib/bats

WORKDIR /work
