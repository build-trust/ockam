ARG BASE_IMAGE=debian:11.1-slim@sha256:312218c8dae688bae4e9d12926704fa9af6f7307a6edb4f66e479702a9af5a0c
FROM ${BASE_IMAGE}

ARG GOSU_VERSION
ARG GOSU_SHA256
ENV GOSU_VERSION=${GOSU_VERSION:-1.12} \
    GOSU_SHA256=${GOSU_SHA256:-0f25a21cf64e58078057adc78f38705163c1d564a959ff30a891c31917011a54}

COPY entrypoint.sh /docker-entrypoint.sh

RUN set -xe; \
    apt-get update; \
    DEBIAN_FRONTEND=noninteractive apt-get install --assume-yes --no-install-recommends \
      ca-certificates \
      curl \
      locales \
      netcat; \
    apt-get clean; \
    rm -rf /var/lib/apt/lists/*; \
# Setup locale
    LANG=en_US.UTF-8; \
    echo $LANG UTF-8 > /etc/locale.gen; \
    locale-gen; \
    update-locale LANG=$LANG; \
# Setup gosu
    curl -o /usr/local/bin/gosu -sSL https://github.com/tianon/gosu/releases/download/${GOSU_VERSION}/gosu-amd64; \
    echo "${GOSU_SHA256}  /usr/local/bin/gosu" | sha256sum -c -; \
    chmod +x /usr/local/bin/gosu; \
# Setup entrypoint
    chmod +x /docker-entrypoint.sh

ENV LANG=en_US.UTF-8

WORKDIR /work

ENTRYPOINT ["/docker-entrypoint.sh"]
