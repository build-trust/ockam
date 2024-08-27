#!/usr/bin/env bash

run() {
    enrollment_ticket="$1"

    # ----------------------------------------------------------------------------------------------------------------
    # CREATE INSTANCE AND START RELAY
    sed "s/\$ENROLLMENT_TICKET/${enrollment_ticket}/g" run_ockam.sh > user_data1.sh
    sed "s/\$OCKAM_VERSION/${OCKAM_VERSION}/g" user_data1.sh > user_data.sh

    gcloud compute instances create "${name}-key" \
        --project="$GOOGLE_CLOUD_PROJECT_ID" \
        --zone="us-central1-c" \
        --create-disk=auto-delete=yes,boot=yes,device-name="${name}-key",image=projects/debian-cloud/global/images/debian-12-bookworm-v20240815,mode=rw,size=10,type=pd-balanced \
        --machine-type=e2-medium \
        --network-interface=network-tier=PREMIUM,stack-type=IPV4_ONLY,subnet=default \
        --tags="${name}-key" \
        --metadata-from-file=startup-script=user_data.sh

    rm -rf user_data*.sh
}

cleanup() {
    # ----------------------------------------------------------------------------------------------------------------
    # DELETE INSTANCE
    gcloud compute instances delete "${name}-key" --zone="us-central1-c" --project="$GOOGLE_CLOUD_PROJECT_ID" --quiet || true
    rm -rf user_data*.sh
}


user=""
command -v sha256sum &>/dev/null && user=$(gcloud auth list --format json | jq -r '.[0].account' | sha256sum | cut -c 1-20)
command -v shasum &>/dev/null && user=$(gcloud auth list --format json | jq -r '.[0].account' | shasum -a 256 | cut -c 1-20)
export name="ockam-ex-ts-m-$user"

# Check if the first argument is "cleanup"
# If it is, call the cleanup function. If not, call the run function.
if [ "$1" = "cleanup" ]; then cleanup; else run "$1"; fi
