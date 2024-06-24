#!/usr/bin/env bash
set -e

# This script, `./run.sh ...` is invoked on a developer’s work machine.
#
# This hands-on example uses Ockam to create an end-to-end encrypted portal
# to an API. We connect a python API Client in one Amazon VPC with a
# python flask API Server in another Amazon VPC.
#
# The example uses AWS CLI to create these VPCs.
#
# You can read a detailed walkthrough of this example at:
# https://docs.ockam.io/portals/apis/python/amazon_ec2

run() {
    # Run `ockam enroll`.
    #
    # The enroll command creates a new vault and generates a cryptographic identity with private keys stored in that
    # vault. It then guides you to sign in to Ockam Orchestrator.
    #
    # If this is your first time signing in, the Orchestrator creates a new dedicated project for you. A project offers
    # two services: a membership authority and a relay service.
    #
    # The enroll command then asks this project’s membership authority to sign and issue a credential that attests that
    # your identifier is a member of this project. Since your account in Orchestrator is the creator and hence first
    # administrator on this new project, the membership authority issues this credential. The enroll command stores the
    # credential for later use and exits.
    ockam enroll

    # Create an enrollment ticket to enroll the identity used by an ockam node that will run adjacent to the api
    # server in monitoring_corp's network.
    #
    # The identity that enrolls with the generated ticket will be given a project membership credential in which the
    # project membership authority will cryptographically attest to specified attributes - monitoring-api-outlet=true.
    #
    # The identity will also allowed to create a relay in the project at the address `monitoring-api`.
    monitoring_corp_ticket=$(ockam project ticket --usage-count 1 --expires-in 60m \
        --attribute "monitoring-api-outlet=true" --relay monitoring-api)

    # Create an enrollment ticket to enroll the identity used by an ockam node that will run adjacent to the
    # client app in travel_app_corp_ticket's network.
    #
    # The identity that enrolls with the generated ticket will be given a project membership credential in which the
    # project membership authority will cryptographically attest to specified attributes - monitoring-api-inlet=true.
    travel_app_corp_ticket=$(ockam project ticket --usage-count 1 --expires-in 60m \
        --attribute "monitoring-api-inlet=true")

    if [[ -n "$OCKAM_VERSION" ]]; then
        export OCKAM_VERSION="v${OCKAM_VERSION}";
    fi

    # Invoke `monitoring_corp/run.sh` in the directory that has monitoring_corp's configuration. Pass the above
    # enrollment ticket as the first argument to run.sh script. Read monitoring_corp/run.sh to understand the parts
    # that are provisioned in monitoring_corp's virtual private cloud.
    echo; pushd monitoring_corp; ./run.sh "$monitoring_corp_ticket"; popd

    # Invoke `travel_app_corp/run.sh` in the directory that has travel_app_corp's configuration. Pass the above
    # enrollment ticket as the first argument to run.sh script. Read travel_app_corp/run.sh to understand the parts
    # that are provisioned in travel_app_corp's virtual private cloud.
    echo; pushd travel_app_corp; ./run.sh "$travel_app_corp_ticket"; popd
}

# Cleanup after the example - `./run.sh cleanup`
# Remove all resources that were created in AWS.
cleanup() {
    pushd monitoring_corp; ./run.sh cleanup; popd
    pushd travel_app_corp; ./run.sh cleanup; popd
}

# Check if Ockam Command is already installed and available in path.
# If it's not, then install it.
if ! type ockam &>/dev/null && ! [[ "$1" = "cleanup" ]]; then
    curl --proto '=https' --tlsv1.2 -sSfL https://install.command.ockam.io | bash
    source "$HOME/.ockam/env"
fi

# Check that tools we need are installed.
for c in aws curl; do
    if ! type "$c" &>/dev/null; then echo "ERROR: Please install: $c" && exit 1; fi
done

# Check if the first argument is "cleanup"
# If it is, call the cleanup function. If not, call the run function.
if [ "$1" = "cleanup" ]; then cleanup; else run; fi
