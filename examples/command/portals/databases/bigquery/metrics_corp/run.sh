#!/usr/bin/env bash


run() {
    enrollment_ticket="$1"
    private_endpoint_address="10.200.0.5"

    # ----------------------------------------------------------------------------------------------------------------
    # CREATE PRIVATE GOOGLE API ENDPOINT (PRIVATE SERVICE CONNECT API)

    # Create a new VPC.
    gcloud compute networks create "${name}-vpc" --subnet-mode=custom --project="$GOOGLE_CLOUD_PROJECT_ID"

    # Create a subnet in the VPC.
    gcloud compute networks subnets create "${name}-subnet" --network="${name}-vpc" --project="$GOOGLE_CLOUD_PROJECT_ID" \
        --range=10.0.0.0/24 --region=us-central1

    # Enable Private Google Access for the subnet.
    gcloud compute networks subnets update "${name}-subnet" --project="$GOOGLE_CLOUD_PROJECT_ID" \
        --region=us-central1 --enable-private-ip-google-access

    # Reserve an internal IP address for the private service connect (psc).
    gcloud compute addresses create "${name}-psc-address" --global --project="$GOOGLE_CLOUD_PROJECT_ID" \
        --purpose=PRIVATE_SERVICE_CONNECT --addresses="$private_endpoint_address" --network="${name}-vpc"

    # Create a forwarding rule to connect to BigQuery using the reserved IP address.
    gcloud compute forwarding-rules create "$PRIVATE_ENDPOINT_NAME" --global --project="$GOOGLE_CLOUD_PROJECT_ID" \
        --network="${name}-vpc" --address="${name}-psc-address" --target-google-apis-bundle=all-apis

    # Allow Egress traffic to the internet.
    gcloud compute firewall-rules create allow-all-egress \
        --network="${name}-vpc" --allow=all --direction=EGRESS --priority=1000 --destination-ranges=0.0.0.0/0 --target-tags=allow-egress


    # ----------------------------------------------------------------------------------------------------------------
    # CREATE INSTANCE USING THE PRIVATE GOOGLE API ENDPOINT
    sed "s/\$ENROLLMENT_TICKET/${enrollment_ticket}/g" run_ockam.sh > user_data1.sh
    sed "s/\$OCKAM_VERSION/${OCKAM_VERSION}/g" user_data1.sh > user_data2.sh
    sed "s/\$PRIVATE_ENDPOINT_NAME/${PRIVATE_ENDPOINT_NAME}/g" user_data2.sh > user_data.sh

    gcloud compute instances create "${name}-vm-instance" \
        --project="$GOOGLE_CLOUD_PROJECT_ID" \
        --zone="us-central1-a" \
        --create-disk=auto-delete=yes,boot=yes,device-name="${name}-vm-instance",image=projects/debian-cloud/global/images/debian-12-bookworm-v20240815,mode=rw,size=10,type=pd-balanced \
        --machine-type=e2-medium \
        --subnet="${name}-subnet" \
        --tags=allow-egress \
        --metadata-from-file=startup-script=user_data.sh

    rm -rf user_data*.sh
}

cleanup() {
    # ----------------------------------------------------------------------------------------------------------------
    # DELETE NETWORK

    # Delete forwarding rule
    gcloud compute forwarding-rules delete "$PRIVATE_ENDPOINT_NAME" --global --quiet

    # Delete reserved endpoint address
    gcloud compute addresses delete "${name}-psc-address" --global --quiet

    # Delete rule to allow egress
    gcloud compute firewall-rules delete allow-all-egress --quiet

    # ----------------------------------------------------------------------------------------------------------------
    # DELETE INSTANCE RESOURCES
    gcloud compute instances delete "${name}-vm-instance" --zone="us-central1-a" --project="$GOOGLE_CLOUD_PROJECT_ID" --quiet

    # Delete subnet
    gcloud compute networks subnets delete "${name}-subnet" --region=us-central1 --quiet
    # Delete VPC
    gcloud compute networks delete "${name}-vpc" --quiet
    
    rm -rf user_data*.sh
}


user=""
command -v sha256sum &>/dev/null && user=$(gcloud auth list --format json | jq -r '.[0].account' | sha256sum | cut -c 1-20)
command -v shasum &>/dev/null && user=$(gcloud auth list --format json | jq -r '.[0].account' | shasum -a 256 | cut -c 1-20)
export name="ockam-ex-ts-m-$user"

# Check if the first argument is "cleanup"
# If it is, call the cleanup function. If not, call the run function.
if [ "$1" = "cleanup" ]; then cleanup; else run "$1"; fi
