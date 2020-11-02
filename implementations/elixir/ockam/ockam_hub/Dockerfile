
FROM ockam/builder:latest as ockam_hub_build
COPY . /opt/ockam
RUN set -xe; \
    cd /opt/ockam; \
    ./gradlew veryClean; \
    ./gradlew build; \
    cd implementations/elixir/ockam/ockam_hub; \
    MIX_ENV=prod mix release

FROM debian:buster-slim@sha256:1be41347adaee8303bf12114b9edf4af0b35a5e1d9756b3ddad59856eaa31ea7 as ockam_hub
RUN set -xe; \
    mkdir -p /opt/ockam; \
    DEBIAN_FRONTEND=noninteractive \
    apt-get update; \
    apt-get install -y --no-install-recommends ca-certificates locales libssl1.1; \
    apt-get clean; \
    rm -rf /var/lib/apt/lists/*; \
# setup locale
    export LANG=en_US.UTF-8; \
    echo $LANG UTF-8 > /etc/locale.gen; \
    locale-gen; \
    update-locale LANG=$LANG;
ENV LANG=en_US.UTF-8
COPY --from=ockam_hub_build \
  /opt/ockam/implementations/elixir/ockam/ockam_hub/_build /opt/ockam/implementations/elixir/ockam/ockam_hub/_build
ENTRYPOINT ["/opt/ockam/implementations/elixir/ockam/ockam_hub/_build/prod/rel/ockam_hub/bin/ockam_hub"]
CMD ["start"]
