
# Build Containers

```
docker build -t ockam/builder -f tools/docker/elixir/builder/Dockerfile .
```

```
docker build --target ockam_hub_build -t ockam/ockam_hub/build -f implementations/elixir/ockam/ockam_hub/Dockerfile .
```

```
docker build --target ockam_hub -t ockam/ockam_hub -f implementations/elixir/ockam/ockam_hub/Dockerfile .
```

# Test

```
docker run -p 4000:4000 --rm -it ockam/ockam_hub:latest
```

# Publish

```
docker push ockam/ockam_hub:latest
```

# Deploy

```
az group create --name ockam_hub --location westus
```

```
az container create --name ockam-hub-1 --image ockam/ockam_hub:latest --resource-group ockam_hub \
  --ip-address public --ports 4000
```

# Destroy

```
az container delete --resource-group ockam_hub --name ockam-hub-1
```
