# Stage 1 - Build elixir release of ockam_healthcheck elixir app
FROM ghcr.io/build-trust/ockam-elixir-builder:latest AS elixir-app-release-build

COPY --from=cgr.dev/chainguard/wolfi-base /bin /bin
COPY --from=cgr.dev/chainguard/wolfi-base /usr/bin /usr/bin

ENV PATH=/root/.cargo/bin:$PATH
COPY . /work
RUN set -ex; \
    cd work; \
    rustup-init --no-update-default-toolchain -y; \
    rustup show; \
    cargo --version; \
    cd implementations/elixir/ockam/ockam_healthcheck; \
    MIX_ENV=prod OCKLY_PRECOMPILED_LIB=false mix local.hex --force; \
    MIX_ENV=prod OCKLY_PRECOMPILED_LIB=false mix local.rebar --force; \
    MIX_ENV=prod OCKLY_PRECOMPILED_LIB=false mix deps.get; \
    MIX_ENV=prod OCKLY_PRECOMPILED_LIB=false mix release;


# Stage 2 - Create container and copy executables in above step
FROM ghcr.io/build-trust/ockam-elixir-base:latest AS app

COPY --from=cgr.dev/chainguard/wolfi-base /bin /bin
COPY --from=cgr.dev/chainguard/wolfi-base /usr/bin /usr/bin

COPY --from=elixir-app-release-build /work/implementations/elixir/ockam/ockam_healthcheck/_build/prod/rel/ockam_healthcheck /opt/ockam_healthcheck

ENV LANG=C.UTF-8

EXPOSE 4000

ENTRYPOINT ["/opt/ockam_healthcheck/bin/ockam_healthcheck"]
CMD ["start"]
