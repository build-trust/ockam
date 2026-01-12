#!/usr/bin/env bash
set -ex

export SET_AWS_REGION="us-west-2"
source ../../common/aws.sh

run() {
    common_create "$1" "../../common/admin.sh" "0"
}

cleanup() {
    common_cleanup
}

export AWS_PAGER="";
export AWS_DEFAULT_OUTPUT="text";

user=""
command -v sha256sum &>/dev/null && user=$(aws sts get-caller-identity | sha256sum | cut -c 1-20)
command -v shasum &>/dev/null && user=$(aws sts get-caller-identity | shasum -a 256 | cut -c 1-20)
export name="ockam-ex-rendezvous-aws-aws-different-regions-bob-$user"

# Check if the first argument is "cleanup"
# If it is, call the cleanup function. If not, call the run function.
if [ "$1" = "cleanup" ]; then cleanup; else run "$1"; fi
