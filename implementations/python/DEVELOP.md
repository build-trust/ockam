## Setup

```
uv venv --python 3.12
source .venv/bin/activate
uv sync
```

## Build

```
make

# Create docker image to test locally
make build-image-amd64
```

## Release

```
# osx library
make build_release

# linux libraries
make release-linux

# Docker image build and push to ghcr.io
make release-image-amd64
```
