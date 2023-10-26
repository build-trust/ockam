# Stage 1 - Build elixir release of ockam_healthcheck elixir app
FROM cgr.dev/chainguard/wolfi-base AS elixir-app-release-build

RUN set -xe; \
    apk add curl xz bash elixir erlang-dev git openssl ca-certificates ncurses gcc gcc-12 glibc-dev libstdc++-12 glibc gcc llvm-libcxx-16

ENV PATH=/root/.cargo/bin:$PATH
COPY . /work
RUN set -xe; \
    cd work; \
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- --default-toolchain none -y; \
    rustup show; \
    cargo --version; \
    cd implementations/elixir/ockam/ockam_healthcheck; \
    MIX_ENV=prod OCKLY_PRECOMPILED_LIB=false mix local.hex --force; \
    MIX_ENV=prod OCKLY_PRECOMPILED_LIB=false mix local.rebar --force; \
    MIX_ENV=prod OCKLY_PRECOMPILED_LIB=false mix deps.get; \
    MIX_ENV=prod OCKLY_PRECOMPILED_LIB=false mix release;


# Stage 2 - Create container and copy executables in above step
FROM cgr.dev/chainguard/wolfi-base AS app

COPY --from=elixir-app-release-build /work/implementations/elixir/ockam/ockam_healthcheck/_build/prod/rel/ockam_healthcheck /opt/ockam_healthcheck

ENV LANG=C.UTF-8

RUN apk add ncurses gcc

EXPOSE 4000

ENTRYPOINT ["/opt/ockam_healthcheck/bin/ockam_healthcheck"]
CMD ["start"]
