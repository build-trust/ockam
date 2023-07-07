FROM gcc:12.3.0@sha256:5e1768ffd3230192358cfbde63443750f8f78f160a449d25007de9828c41976d

# use unstable packages only when necessary
COPY apt-preference /etc/apt/preferences.d/stable

RUN set -xe; \
# Update base system, useful to update
    apt update; \
    DEBIAN_FRONTEND=noninteractive apt upgrade --assume-yes -y; \
    DEBIAN_FRONTEND=noninteractive apt install --assume-yes build-essential apt-utils -y; \
# Setup Tauri dependencies \
    echo "deb http://deb.debian.org/debian experimental main" >> /etc/apt/sources.list; \
    echo "deb http://deb.debian.org/debian sid main" >> /etc/apt/sources.list; \
    apt update;
RUN set -xe; \
    DEBIAN_FRONTEND=noninteractive apt install --assume-yes libgtk-4-dev vite --no-install-recommends  -y;
RUN set -xe; \
    DEBIAN_FRONTEND=noninteractive apt install javascriptcoregtk-4.1 libsoup-3.0 webkit2gtk-4.1 --no-install-recommends  -y; \
    npm install -g pnpm@${PNPM_VERSION}; \
    npm install -g vite@${VITE_VERSION}; \
    cargo install --locked tauri-cli; \
# Cleanup
    apt clean; \
    rm -rf /var/lib/apt/lists/*

WORKDIR /work
