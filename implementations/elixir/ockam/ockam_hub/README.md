
# Build Containers.

docker build -t ockam/builder -f tools/docker/elixir/builder/Dockerfile .
docker build --target ockam_hub_build -t ockam/ockam_hub/build -f implementations/elixir/ockam/ockam_hub/Dockerfile .
docker build --target ockam_hub -t ockam/ockam_hub -f implementations/elixir/ockam/ockam_hub/Dockerfile .
