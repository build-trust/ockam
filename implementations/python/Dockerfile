FROM ghcr.io/build-trust/ockam-python-dev:latest AS dev

FROM cgr.dev/chainguard/python:latest
WORKDIR /app
COPY --from=dev /app/venv /app/venv
ENV PATH="/app/venv/bin:$PATH"
