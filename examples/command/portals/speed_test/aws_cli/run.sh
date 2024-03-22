#!/usr/bin/env bash
set -e

# This script, `./run.sh ...` is invoked on a developer’s work machine.
#
# The example uses AWS CLI to create these VPCs.
#

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

    # Create an enrollment ticket to enroll the identity used by an ockam node that will run adjacent to the iperf
    # in server's network.
    #
    # The identity that enrolls with the generated ticket will be given a project membership credential in which the
    # project membership authority will cryptographically attest to the specified attributes - iperf-outlet=true.
    #
    # The identity will also allowed to create a relay in the project at the address `iperf`.
    server_ticket=$(ockam project ticket --usage-count 1 --expires-in 60m \
        --attribute "iperf-outlet=true" --relay iperf)

    # Create an enrollment ticket to enroll the identity used by an ockam node that will run adjacent to the iperf
    # client in client's network.
    #
    # The identity that enrolls with the generated ticket will be given a project membership credential in which the
    # project membership authority will cryptographically attest to the specified attributes - iperf-inlet=true.
    client_ticket=$(ockam project ticket --usage-count 1 --expires-in 60m \
        --attribute "iperf-inlet=true")

    # Invoke `server/run.sh` in the directory that has server's configuration. Pass the above enrollment ticket
    # as the first argument to run.sh script. Read server/run.sh to understand the parts that are provisioned in
    # server's virtual private cloud.
    echo; pushd server; ./run.sh "$server_ticket"; popd

    # Invoke `client/run.sh` in the directory that has client's configuration. Pass the above enrollment
    # ticket as the first argument to run.sh script. Read client/run.sh to understand the parts that are
    # provisioned in client's virtual private cloud.
    echo; pushd client; ./run.sh "$client_ticket"; popd

    echo "To view the server iperf: server/attach.sh"
    echo "To start the speed test run: client/speed_test.sh"
}

# Cleanup after the example - `./run.sh cleanup`
# Remove all resources that were created in AWS.
cleanup() {
    pushd client; ./run.sh cleanup; popd
    pushd server; ./run.sh cleanup; popd
}

# Check if Ockam Command is already installed and available in path.
# If it's not, then install it.
if ! type ockam &>/dev/null; then
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
