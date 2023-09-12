# Stage 1 - Build elixir release of ockam_healthcheck elixir app
FROM ghcr.io/build-trust/ockam-builder@sha256:5ab42598e35509cad3ea9c1e1bd0ed135ed1340c6ae44b762b1c8bbec31d5c68 as elixir-app-release-build
COPY . /work
RUN set -xe; \
    cd implementations/elixir; \
    make build; \
    cd ockam/ockam_healthcheck; \
    MIX_ENV=prod mix release;


# TODO: Use distroless container after https://github.com/elixir-lang/elixir/issues/11942 is closed
# Stage 2 - Create container and copy executables in above step
FROM debian:11.1-slim@sha256:312218c8dae688bae4e9d12926704fa9af6f7307a6edb4f66e479702a9af5a0c

COPY --from=elixir-app-release-build /work/implementations/elixir/ockam/ockam_healthcheck/_build/prod/rel/ockam_healthcheck /opt/ockam_healthcheck

ENV LANG=C.UTF-8

EXPOSE 4000

ENTRYPOINT ["/opt/ockam_healthcheck/bin/ockam_healthcheck"]
CMD ["start"]
