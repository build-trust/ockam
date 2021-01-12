
## Base

```
docker build \
  --build-arg BASE_IMAGE=debian:10.7-slim@sha256:240f770008bdc538fecc8d3fa7a32a533eac55c14cbc56a9a8a6f7d741b47e33 \
  --tag ockam/base:latest base
```

## Builder Base

```
docker build \
  --build-arg BASE_IMAGE=gcc:9.3.0@sha256:488373ff1b96186d48ea47f9c5eb0495b87a2ac990d15248d64d605ac7bdb539 \
  --tag ockam/builder_base:latest base
```

## Builder

Build the builder:

```
docker build \
  --build-arg BASE_IMAGE=ockam/builder_base:latest \
  --tag ockam/builder:latest builder
```

Run the builder:

```
docker run --rm -it -e LOCAL_USER_ID=$(id -u) --volume $(pwd):/work ockam/builder:latest bash
```
