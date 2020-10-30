FROM debian:buster-slim@sha256:1be41347adaee8303bf12114b9edf4af0b35a5e1d9756b3ddad59856eaa31ea7

RUN set -xe; \
    apt-get update; \
    DEBIAN_FRONTEND=noninteractive \
    apt-get install --assume-yes --no-install-recommends ca-certificates locales gnupg dirmngr curl make gcc g++ git; \
    apt-get clean; \
    rm -rf /var/lib/apt/lists/*; \
# setup locale
    export LANG=en_US.UTF-8; \
    echo $LANG UTF-8 > /etc/locale.gen; \
    locale-gen; \
    update-locale LANG=$LANG;

ENV LANG=en_US.UTF-8 \
    RUSTUP_HOME=/usr/local/rustup \
    CARGO_HOME=/usr/local/cargo \
    RUST_VERSION=1.47.0 \
    JAVA_HOME=/opt/java/openjdk \
    PATH=/opt/java/openjdk/bin:/usr/local/cargo/bin:/opt/cmake-3.17.3-Linux-x86_64/bin:$PATH \
    AR=/usr/bin/ar \
    AS=/usr/bin/as \
    CC=/usr/bin/gcc \
    CPP=/usr/bin/cpp \
    CXX=/usr/bin/g++

RUN set -xe; \
# setup cmake
    curl -sSOL 'https://github.com/Kitware/CMake/releases/download/v3.17.3/cmake-3.17.3-Linux-x86_64.tar.gz'; \
    echo 'da8093956f0b4ae30293c9db498da9bdeaeea4e7a2b1f2d1637ddda064d06dd0  cmake-3.17.3-Linux-x86_64.tar.gz' | \
      sha256sum -c -; \
    tar -xf cmake-3.17.3-Linux-x86_64.tar.gz -C /opt/; \
    rm -rf cmake-3.17.3-Linux-x86_64.tar.gz; \
# setup rust
    curl -sSOL 'https://static.rust-lang.org/rustup/archive/1.22.1/x86_64-unknown-linux-gnu/rustup-init'; \
    echo '49c96f3f74be82f4752b8bffcf81961dea5e6e94ce1ccba94435f12e871c3bdb  rustup-init' | sha256sum -c -; \
    chmod +x rustup-init; \
    ./rustup-init -y --no-modify-path --profile minimal \
      --default-toolchain "$RUST_VERSION" --default-host x86_64-unknown-linux-gnu; \
    rm rustup-init; \
    chmod -R a+w "$RUSTUP_HOME" "$CARGO_HOME"; \
# setup erlang
    curl -sSOL 'https://packages.erlang-solutions.com/erlang-solutions_2.0_all.deb'; \
    echo 'a191b37f11d8133c12ae146d76072f5f1d884d80e1f1bccd8c7aeb55f70a72cb  erlang-solutions_2.0_all.deb' | \
      sha256sum -c -; \
    dpkg -i erlang-solutions_2.0_all.deb; \
    rm -rf erlang-solutions_2.0_all.deb; \
# setup elixir
    apt-get update; \
    apt-get install --assume-yes --no-install-recommends esl-erlang; \
    curl -sSOL 'https://packages.erlang-solutions.com/erlang/debian/pool/elixir_1.10.4-1~debian~buster_all.deb'; \
    echo '07f2ac5980f856353e2c7d3f04f16176c9ad8967ccfa39a6e07cf676c704807f  elixir_1.10.4-1~debian~buster_all.deb' | \
      sha256sum -c; \
    dpkg -i 'elixir_1.10.4-1~debian~buster_all.deb'; \
    rm -rf 'elixir_1.10.4-1~debian~buster_all.deb'; \
    mix local.hex --force && mix local.rebar --force; \
# setup jdk
    cd /tmp; \
    curl -sSOL 'https://github.com/AdoptOpenJDK/openjdk11-binaries/releases/download/jdk11u-2020-10-23-08-51/OpenJDK11U-jdk_x64_linux_hotspot_2020-10-23-08-51.tar.gz'; \
    echo '66e8c7bc970b7c9873a3ab706944c6b4e2c49dd75e28f9b38ebe2bbb267a1ad8  OpenJDK11U-jdk_x64_linux_hotspot_2020-10-23-08-51.tar.gz'; \
    mv 'OpenJDK11U-jdk_x64_linux_hotspot_2020-10-23-08-51.tar.gz' openjdk.tar.gz; \
    mkdir -p /opt/java/openjdk; \
    cd /opt/java/openjdk; \
    tar -xf /tmp/openjdk.tar.gz --strip-components=1; \
    rm -rf /tmp/openjdk.tar.gz;
