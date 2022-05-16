ARG BUILDER_IMAGE=ghcr.io/build-trust/ockam-builder@sha256:3ea4c222da220c69b62a60d42d83769286e07bf8e41ef7cfdbeba18b7f22a488
ARG BASE_IMAGE=ghcr.io/build-trust/ockam-base@sha256:40fcb081b6cf56d1e306d859d010a8a4c7b9a02e6b9bc468848c09653f714b74

# Stage 1
FROM ${BUILDER_IMAGE}
COPY . /work
RUN set -xe; \
    cd implementations/elixir; \
    ../../gradlew build; \
    cd ockam/ockam_cloud_node; \
    MIX_ENV=prod mix release;

# Stage 2
FROM ${BASE_IMAGE}
COPY --from=0 /work/implementations/elixir/ockam/ockam_cloud_node/_build/prod/rel/ockam_cloud_node /opt/ockam_cloud_node

ENTRYPOINT ["/opt/ockam_cloud_node/bin/ockam_cloud_node"]
CMD ["start"]
