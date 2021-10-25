ARG BUILDER_IMAGE=ghcr.io/ockam-network/ockam/builder:latest
ARG BASE_IMAGE=ghcr.io/ockam-network/ockam/base:latest

# Stage 1
FROM ${BUILDER_IMAGE}
COPY . /work
RUN set -xe; \
    cd implementations/elixir; \
    ../../gradlew build; \
    cd ockam/ockam_hub; \
    MIX_ENV=prod mix release;

# Stage 2
FROM ${BASE_IMAGE}
COPY --from=0 /work/implementations/elixir/ockam/ockam_hub/_build/prod/rel/ockam_hub /opt/ockam_hub

ENTRYPOINT ["/opt/ockam_hub/bin/ockam_hub"]
CMD ["start"]
