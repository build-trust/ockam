
FROM ockam/builder:latest as ockam_hub_build
COPY . /ockam
RUN set -xe;\
    cd /ockam; \
    ./gradlew clean; \
    ./gradlew build; \
    cd implementations/elixir/ockam/ockam_hub; \
    MIX_ENV=prod mix release

FROM debian:buster-slim@sha256:1be41347adaee8303bf12114b9edf4af0b35a5e1d9756b3ddad59856eaa31ea7 as ockam_hub
RUN mkdir -p /opt/ockam
COPY --from=ockam_hub_build /ockam/implementations/elixir/ockam/ockam_hub/_build/prod/rel/ockam_hub /opt/ockam/ockam_hub
