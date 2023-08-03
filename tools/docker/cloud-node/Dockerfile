# Stage 1 - Build elixir release of ockam_cloud_node elixir app
FROM ghcr.io/build-trust/ockam-builder@sha256:9ee4558d834514e60a50c41bbf38c6ecae47d94dcfaa4df6a7256c262a7b0f4b as elixir-app-release-build
COPY . /work
RUN set -xe; \
    cd implementations/elixir; \
    ../../gradlew build; \
    cd ockam/ockam_cloud_node; \
    MIX_ENV=prod mix release;


# TODO: Use distroless container after https://github.com/elixir-lang/elixir/issues/11942 is closed
# Stage 2 - Create container and copy executables in above step
FROM debian:12.1-slim@sha256:5bbfcb9f36a506f9c9c2fb53205f15f6e9d1f0e032939378ddc049a2d26d651e

COPY --from=elixir-app-release-build /work/implementations/elixir/ockam/ockam_cloud_node/_build/prod/rel/ockam_cloud_node /opt/ockam_cloud_node

ENV LANG=C.UTF-8

EXPOSE 4000

ENTRYPOINT ["/opt/ockam_cloud_node/bin/ockam_cloud_node"]
CMD ["start"]
