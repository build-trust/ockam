# Stage 1 - Build elixir release of ockam_healthcheck elixir app
FROM ghcr.io/build-trust/ockam-elixir-builder:latest

COPY --from=cgr.dev/chainguard/wolfi-base /bin /bin
COPY --from=cgr.dev/chainguard/wolfi-base /usr/bin /usr/bin

ENV PATH=/root/.cargo/bin:$PATH
COPY . /work
RUN set -ex; \
    ls; \
    cd work; \
    rustup-init --no-update-default-toolchain -y; \
    rustup show; \
    cargo --version; \
    OCKAM_BUILD_NIF=true make elixir_build_ockly;

CMD ["sh"]
