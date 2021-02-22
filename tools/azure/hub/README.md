# Publish

```
docker push ghcr.io/ockam-network/ockam/hub:latest
```

# Deploy

Set enviromental variables
```
AZ_RESOURCE_GROUP=ockam-hub
AZ_DEPLOYMENT=ockam-hub-deployment
```

Create the resource group
```
az group create --name $AZ_RESOURCE_GROUP --location westus
```

Create the container group and its only container
```
az deployment group create \
  --name $AZ_DEPLOYMENT \
  --resource-group $AZ_RESOURCE_GROUP \
  --template-file tools/azure/hub/azure.json \
  --parameters @tools/azure/hub/secret.parameters.json
```

Attach to container
```
az container attach --name ockam-hub --resource-group $AZ_RESOURCE_GROUP
```

Shell into container
```
az container exec \
  --name ockam-hub \
  --resource-group $AZ_RESOURCE_GROUP \
  --exec-command "/bin/bash"
```

> If your terminal size is different than standard 80x24 when typing inside your container you can expirence bizzare behaviour. To fix this use additional parameters like `--terminal-col-size` and `--terminal-row-size`.
>
```
az container exec \
  --name ockam-hub \
  --resource-group $AZ_RESOURCE_GROUP \
  --exec-command "/bin/bash" \
  --terminal-col-size $(tput cols) \
  --terminal-row-size $(tput lines)
```
>
> If you missing `tput` command then follow [this](https://command-not-found.com/tput) instructions to install it.

Show container IP
```
az container show \
  --name ockam-hub \
  --resource-group $AZ_RESOURCE_GROUP \
  --query ipAddress.ip \
  --output tsv
```

Delete container
```
az container delete --name ockam-hub --resource-group $AZ_RESOURCE_GROUP
```

Delete resource group
```
az group delete --name $AZ_RESOURCE_GROUP
```
