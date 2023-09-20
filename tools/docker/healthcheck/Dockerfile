# Stage 1 - Build elixir release of ockam_healthcheck elixir app
FROM hexpm/elixir:1.14.5-erlang-24.3.4.11-debian-bullseye-20230227-slim@sha256:69a2436202ac4cd0078dff0758d11e0c55ed663fc8de8bdc9c7287a369912db4 AS elixir-app-release-build

# install build dependencies
# FIXME: we should use precompile .so instead of requiring the rust compiler
RUN apt-get update; \
  apt-get install -y --no-install-recommends \
  ca-certificates \
  openssl \
  libncurses5 \
  git \
  curl \
  g++

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- --default-toolchain 1.71.1 -y
ENV PATH=/root/.cargo/bin:$PATH

COPY . /work
RUN set -xe; \
    cd /work/implementations/elixir/ockam/ockam_healthcheck; \
    MIX_ENV=prod OCKLY_PRECOMPILED_LIB=false mix local.hex --force; \
    MIX_ENV=prod OCKLY_PRECOMPILED_LIB=false mix local.rebar --force; \
    MIX_ENV=prod OCKLY_PRECOMPILED_LIB=false mix deps.get; \
    MIX_ENV=prod OCKLY_PRECOMPILED_LIB=false mix release;

# TODO: Use distroless container after https://github.com/elixir-lang/elixir/issues/11942 is closed
# Stage 2 - Create container and copy executables in above step
FROM debian:bullseye-20230227-slim@sha256:403e06393d6b9dcb506eeef2adba9e30a97139c54e4c90d55254049f7d224081 AS app

COPY --from=elixir-app-release-build /work/implementations/elixir/ockam/ockam_healthcheck/_build/prod/rel/ockam_healthcheck /opt/ockam_healthcheck

ENV LANG=C.UTF-8

EXPOSE 4000

ENTRYPOINT ["/opt/ockam_healthcheck/bin/ockam_healthcheck"]
CMD ["start"]
