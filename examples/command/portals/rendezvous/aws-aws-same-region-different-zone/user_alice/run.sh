#!/usr/bin/env bash
set -ex

# Create AWS environment
export ENABLE_SSH="1"
source ../../common/aws.sh

run() {
    ip=$(common_create "$1" "../../common/user.sh" "1")

    echo "IP address is $ip"
    until nc -z -v -w5 $ip 22; do sleep 5; done

    ssh -o StrictHostKeyChecking=no -i ./key.pem "ec2-user@$ip" \
        'bash -s' << 'EOS'
            sleep 8s
            curl --silent --show-error --fail 127.0.0.1:4000
EOS
}

cleanup() {
    common_cleanup
}

export AWS_PAGER="";
export AWS_DEFAULT_OUTPUT="text";

user=""
command -v sha256sum &>/dev/null && user=$(aws sts get-caller-identity | sha256sum | cut -c 1-20)
command -v shasum &>/dev/null && user=$(aws sts get-caller-identity | shasum -a 256 | cut -c 1-20)
export name="ockam-ex-rendezvous-aws-aws-same-region-different-zone-alice-$user"

# Check if the first argument is "cleanup"
# If it is, call the cleanup function. If not, call the run function.
if [ "$1" = "cleanup" ]; then cleanup; else run "$1"; fi
