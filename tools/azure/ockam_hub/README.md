# Publish

```
docker push ockam/ockam_hub:latest
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
  --template-file tools/azure/ockam_hub/azure.json \
  --parameters @tools/azure/ockam_hub/secret.parameters.json
```

Attach to container

```
az container attach --name ockam-hub --resource-group ockam-hub
```

Shell into container

```
az container exec --name ockam-hub --resource-group ockam-hub --exec-command "/bin/bash"
```

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
