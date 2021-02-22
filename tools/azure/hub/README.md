# Publish

```
docker push ghcr.io/ockam-network/ockam/hub:latest
```

# Deploy

Create the resource group

```
az group create --name ockam-hub --location westus
```

Create the container group and its only container

```
az deployment group create \
  --name ockam-hub-deployment \
  --resource-group ockam-hub \
  --template-file tools/azure/hub/azure.json \
  --parameters @tools/azure/hub/secret.parameters.json
```

Attach to container

```
az container attach --name ockam-hub --resource-group ockam-hub
```

Shell into container

```
az container exec --name ockam-hub --resource-group ockam-hub --exec-command "/bin/bash"
```

> If your terminal size is different than standard 80x24 when typing inside your container you can expirence bizzare behaviour. To fix this use additional parameters like `--terminal-col-size` and `--terminal-row-size`.
>
`az container exec --name ockam-hub --resource-group piotrek --exec-command "/bin/bash" --terminal-col-size $(tput cols) --terminal-row-size $(tput lines)`
>
> If you missing `tput` command then follow [this](https://command-not-found.com/tput) instructions to install it.

Show container IP

```
az container show --name ockam-hub --resource-group ockam-hub --query ipAddress.ip --output tsv
```

Delete container

```
az container delete --name ockam-hub --resource-group ockam-hub
```

Delete resource group

```
az group delete --name ockam-hub
```
