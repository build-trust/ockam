#!/usr/bin/env bash
set -ex
run() {
    enrollment_ticket="$1"

    # Create Resource Group
    echo "Creating Azure Resource Group..."
    az group create --name $RESOURCE_GROUP --location $LOCATION

    # Create Virtual Network
    echo "Creating Azure Virtual Network..."
    az network vnet create \
        --resource-group $RESOURCE_GROUP \
        --name $VNET_NAME \
        --address-prefix "10.0.0.0/16" \
        --subnet-name $SUBNET_NAME \
        --subnet-prefix "10.0.0.0/24" \
        --location $LOCATION

    # Create OpenAI Service
    echo "Creating Azure OpenAI Service..."
    az cognitiveservices account create \
        --name $SERVICE_NAME \
        --resource-group $RESOURCE_GROUP \
        --kind OpenAI \
        --sku S0 \
        --location $LOCATION \
        --custom-domain $SERVICE_NAME \
        --tags "CreatedBy=AzureCLI"

    # Retrieve virtual network and subnet ID
    echo "Retrieving virtual network and subnet ID..."
    VNET_ID=$(az network vnet show --name $VNET_NAME --resource-group $RESOURCE_GROUP --query "id" -o tsv)
    SUBNET_ID=$(az network vnet subnet show --name $SUBNET_NAME --vnet-name $VNET_NAME --resource-group $RESOURCE_GROUP --query "id" -o tsv)

    # Create private endpoint for the OpenAI service
    echo "Creating private endpoint for the OpenAI service..."
    az network private-endpoint create \
        --name $PRIVATE_ENDPOINT_NAME \
        --resource-group $RESOURCE_GROUP \
        --vnet-name $VNET_NAME \
        --subnet $SUBNET_NAME \
        --private-connection-resource-id $(az cognitiveservices account show --name $SERVICE_NAME --resource-group $RESOURCE_GROUP --query "id" -o tsv) \
        --group-id "account" \
        --connection-name "${SERVICE_NAME}-connection"

    # Create private DNS zone and linking it...
    echo "Creating private DNS zone and linking it..."
    az network private-dns zone create \
        --resource-group $RESOURCE_GROUP \
        --name $PRIVATE_DNS_ZONE

    # Link private DNS zone to virtual network...
    echo "Linking private DNS zone to virtual network..."
    az network private-dns link vnet create \
        --resource-group $RESOURCE_GROUP \
        --zone-name $PRIVATE_DNS_ZONE \
        --name "${VNET_NAME}-link" \
        --virtual-network $VNET_ID \
        --registration-enabled false

    # Configure DNS records...
    echo "Configuring DNS records..."
    PRIVATE_IP=$(az network private-endpoint show --name $PRIVATE_ENDPOINT_NAME --resource-group $RESOURCE_GROUP --query "customDnsConfigs[0].ipAddresses[0]" -o tsv)

    # Create DNS record set...
    echo "Creating DNS record set..."
    az network private-dns record-set a create \
        --name $SERVICE_NAME \
        --zone-name $PRIVATE_DNS_ZONE \
        --resource-group $RESOURCE_GROUP

    # Add DNS record...
    echo "Adding DNS record..."
    az network private-dns record-set a add-record \
        --record-set-name $SERVICE_NAME \
        --zone-name $PRIVATE_DNS_ZONE \
        --resource-group $RESOURCE_GROUP \
        --ipv4-address $PRIVATE_IP

    # Update network ACLs for the OpenAI service to deny public access...
    echo "Updating network ACLs for the OpenAI service to deny public access..."
    RESOURCE_ID=$(az cognitiveservices account show \
        --name $SERVICE_NAME \
        --resource-group $RESOURCE_GROUP \
        --query id -o tsv)

    # Update network ACLs for the OpenAI service to deny public access...
    az resource update \
        --ids $RESOURCE_ID \
        --set properties.networkAcls="{'defaultAction':'Deny'}" \
        --set properties.publicNetworkAccess="Disabled"

    # Deploy model on OpenAI service
    echo "Deploying model ${MODEL_NAME} on ${SERVICE_NAME}..."
    az cognitiveservices account deployment create \
        --name "$SERVICE_NAME" \
        --resource-group "$RESOURCE_GROUP" \
        --deployment-name "$DEPLOYMENT_NAME" \
        --model-name "$MODEL_NAME" \
        --model-version "$MODEL_VERSION" \
        --model-format OpenAI \
        --sku-capacity 1 \
        --sku-name Standard

    # Setting up VM. Processing Ockam setup script...
    echo "Setting up VM. Processing Ockam setup script..."

    # Replace variables in the script
    sed "s|\${SERVICE_NAME}|${SERVICE_NAME}|g" run_ockam.sh | \
        sed "s|\${TICKET}|${enrollment_ticket}|g" > run_ockam_processed.sh

    chmod +x run_ockam_processed.sh

    # Create VM and run Ockam setup script
    az vm create \
        --resource-group $RESOURCE_GROUP \
        --name "${SERVICE_NAME}-vm" \
        --image "RedHat:RHEL:8-lvm-gen2:latest" \
        --admin-username "azureuser" \
        --generate-ssh-keys \
        --vnet-name $VNET_NAME \
        --subnet $SUBNET_NAME \
        --location $LOCATION \
        --custom-data run_ockam_processed.sh \
        --tags "CreatedBy=AzureCLI"

    # Get API key for OpenAI service
    API_KEY=$(az cognitiveservices account keys list \
        --name $SERVICE_NAME \
        --resource-group $RESOURCE_GROUP \
        --query "key1" \
        -o tsv)

    # Get Ockam project ID
    PROJECT_ID=$(ockam project show --jq .id | tr -d '"')

    # Set Azure OpenAI endpoint
    AZURE_OPENAI_ENDPOINT="https://az-openai.${PROJECT_ID}.ockam.network:443"

    echo "AZURE_OPENAI_ENDPOINT=$AZURE_OPENAI_ENDPOINT" > .env.azure
    echo "AZURE_OPENAI_API_KEY=$API_KEY" >> .env.azure
}

cleanup() {
    echo "Cleaning up..."

    echo "Deleting VM and associated resources..."
    az vm delete \
        --resource-group $RESOURCE_GROUP \
        --name "${SERVICE_NAME}-vm" \
        --yes \
        --force "yes"

    echo "Deleting disk..."
    DISK_NAME=$(az disk list \
        --resource-group $RESOURCE_GROUP \
        --query "[?contains(name, '${SERVICE_NAME}-vm')].name" \
        -o tsv)

    if [ -n "$DISK_NAME" ]; then
        echo "Found disk: $DISK_NAME"
        az disk delete \
            -g "$RESOURCE_GROUP" \
            -n "$DISK_NAME" \
            --yes
    fi

    echo "Deleting NIC..."
    az network nic delete \
        --resource-group $RESOURCE_GROUP \
        --name "${SERVICE_NAME}-vmVMNic"

    echo "Deleting public IP..."
    az network public-ip delete \
        --resource-group $RESOURCE_GROUP \
        --name "${SERVICE_NAME}-vmPublicIP"

    echo "Deleting NSG..."
    az network nsg delete \
        --resource-group $RESOURCE_GROUP \
        --name "${SERVICE_NAME}-vmNSG"

    echo "Deleting Private Endpoint..."
    az network private-endpoint delete \
        --name $PRIVATE_ENDPOINT_NAME \
        --resource-group $RESOURCE_GROUP

    echo "Deleting Azure OpenAI Service..."
    az cognitiveservices account delete \
        --name $SERVICE_NAME \
        --resource-group $RESOURCE_GROUP

    echo "Purging Azure OpenAI Service..."
    az cognitiveservices account purge \
        --name $SERVICE_NAME \
        --resource-group $RESOURCE_GROUP \
        --location $LOCATION \
        || echo "Purge not needed or already completed"

    echo "Deleting Virtual Network..."
    az network vnet delete \
        --name $VNET_NAME \
        --resource-group $RESOURCE_GROUP

    echo "Deleting Resource Group..."
    az group delete \
        --name $RESOURCE_GROUP \
        --yes

    echo "Deleting local files"
    rm run_ockam_processed.sh
    rm .env.azure
}


user=""
command -v sha256sum &>/dev/null && user=$(az ad signed-in-user show --query userPrincipalName -o tsv | sha256sum | cut -c 1-20)
command -v shasum &>/dev/null && user=$(az ad signed-in-user show --query userPrincipalName -o tsv | shasum -a 256 | cut -c 1-20)
export name="ockam-ex-az-openai-aicorp-$user"

export RESOURCE_GROUP="${name}-rg"
export LOCATION="eastus"

# VNet
export VNET_NAME="${name}-vnet"
export SUBNET_NAME="${name}-subnet"

# OpenAI Service
export SERVICE_NAME="${name}-openai"
export PRIVATE_DNS_ZONE="privatelink.openai.azure.com"
export PRIVATE_ENDPOINT_NAME="${SERVICE_NAME}-private-endpoint"

# Model
export DEPLOYMENT_NAME="gpt-4o-mini"
export MODEL_NAME="gpt-4o-mini"
export MODEL_VERSION="2024-07-18"


# Check if the first argument is "cleanup"
# If it is, call the cleanup function. If not, call the run function.
if [ "$1" = "cleanup" ]; then cleanup; else run "$1"; fi
