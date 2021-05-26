
From the root directory of the ockam codebase:

## Base

```
docker build \
  --build-arg BASE_IMAGE=debian:10.7-slim@sha256:240f770008bdc538fecc8d3fa7a32a533eac55c14cbc56a9a8a6f7d741b47e33 \
  --tag ockam/base:latest \
  --tag ghcr.io/ockam-network/ockam/base:latest \
  tools/docker/base
```

## Builder Base

```
docker build \
  --build-arg BASE_IMAGE=gcc:9.3.0@sha256:488373ff1b96186d48ea47f9c5eb0495b87a2ac990d15248d64d605ac7bdb539 \
  --tag ockam/builder_base:latest \
  --tag ghcr.io/ockam-network/ockam/builder_base:latest \
  tools/docker/base
```

## Builder

Build the builder:

```
docker build \
  --build-arg BASE_IMAGE=ockam/builder_base:latest \
  --tag ockam/builder:latest \
  --tag ghcr.io/ockam-network/ockam/builder:latest \
  tools/docker/builder
```

Run the builder:

```
docker run --rm -it -e HOST_USER_ID=$(id -u) --volume $(pwd):/work ockam/builder:latest bash
```

## Hub

```
docker build \
  --tag ockam/hub:latest \
  --tag ghcr.io/ockam-network/ockam/hub:latest \
  --file tools/docker/hub/Dockerfile .
```

Run the hub:

```
docker run --rm -it ockam/hub:latest
```
