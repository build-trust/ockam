# Stage 1 - Build elixir release of ockam_healthcheck elixir app
FROM ghcr.io/build-trust/ockam-elixir-builder@sha256:65bf57734875c150e7dd94cd3b4df256abfab1c1509c9d6ede15b791c2b76cd9 AS elixir-app-release-build

COPY --from=cgr.dev/chainguard/wolfi-base /bin /bin
COPY --from=cgr.dev/chainguard/wolfi-base /usr/bin /usr/bin

# We will be recompiling NIF till 118 release
# ENV OCKAM_DOWNLOAD_NIF="1"
ENV PATH=/root/.cargo/bin:$PATH
COPY . /work
RUN set -ex; \
    cd work; \
    rustup-init --no-update-default-toolchain -y; \
    rustup show; \
    cargo --version; \
    cd implementations/elixir/ockam/ockam_healthcheck; \
    MIX_ENV=prod mix local.hex --force; \
    MIX_ENV=prod mix local.rebar --force; \
    MIX_ENV=prod mix deps.get; \
    MIX_ENV=prod mix release;


# Stage 2 - Create container and copy executables in above step
FROM ghcr.io/build-trust/ockam-elixir-base@sha256:50da5804c0aef8f98b8500f4fcbe877b95826d587d8ef0f474a8895effe6140b AS app

COPY --from=cgr.dev/chainguard/wolfi-base /bin /bin
COPY --from=cgr.dev/chainguard/wolfi-base /usr/bin /usr/bin

COPY --from=elixir-app-release-build /work/implementations/elixir/ockam/ockam_healthcheck/_build/prod/rel/ockam_healthcheck /opt/ockam_healthcheck

ENV LANG=C.UTF-8

EXPOSE 4000

ENTRYPOINT ["/opt/ockam_healthcheck/bin/ockam_healthcheck"]
CMD ["start"]
