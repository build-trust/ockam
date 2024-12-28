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
        --address-prefix "192.168.0.0/16" \
        --subnet-name $SUBNET_NAME \
        --subnet-prefix "192.168.0.0/24" \
        --location $LOCATION

    # Process Ockam setup script...
    echo "Processing Ockam setup script..."

    sed "s|\${SERVICE_NAME}|${SERVICE_NAME}|g" run_ockam.sh | \
        sed "s|\${TICKET}|${enrollment_ticket}|g" > run_ockam_processed.sh

    chmod +x run_ockam_processed.sh

    # Create VM and run Ockam setup script
    echo "Creating VM..."
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

    PUBLIC_IP=$(az vm show \
        --resource-group $RESOURCE_GROUP \
        --name "${SERVICE_NAME}-vm" \
        --show-details \
        --query "publicIps" \
        -o tsv)
    # Copy client.py and .env.azure to VM
    until scp -o StrictHostKeyChecking=no ./client.py "azureuser@$PUBLIC_IP:~/client.py"; do sleep 10; done
    until scp -o StrictHostKeyChecking=no ../ai_corp/.env.azure "azureuser@$PUBLIC_IP:~/.env.azure"; do sleep 10; done

    # Run client.py script on VM
    ssh -o StrictHostKeyChecking=no "azureuser@$PUBLIC_IP" \
            'bash -s' << 'EOS'
                sudo dnf install -y python39 python39-pip
                python3.9 -m pip install --user openai
                export $(cat .env.azure | xargs) && python3.9 /home/azureuser/client.py
EOS

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
}


user=""
command -v sha256sum &>/dev/null && user=$(az ad signed-in-user show --query userPrincipalName -o tsv | sha256sum | cut -c 1-20)
command -v shasum &>/dev/null && user=$(az ad signed-in-user show --query userPrincipalName -o tsv | shasum -a 256 | cut -c 1-20)
export name="ockam-ex-openai-healthcorp-$user"

export RESOURCE_GROUP="${name}-rg"
export LOCATION="eastus"

# VNet
export VNET_NAME="${name}-vnet"
export SUBNET_NAME="${name}-subnet"

# Service
export SERVICE_NAME="${name}-openai"

# Check if the first argument is "cleanup"
# If it is, call the cleanup function. If not, call the run function.
if [ "$1" = "cleanup" ]; then cleanup; else run "$1"; fi
