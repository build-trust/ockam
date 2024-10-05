#!/usr/bin/env bash
set -ex

# PROD RENDEZVOUS
export OCKAM_RENDEZVOUS_SERVER="rendezvous.orchestrator.ockam.io:4000"

run() {
    ockam enroll

    bob_ticket=$(ockam project ticket --usage-count 1 --expires-in 100h --relay bob)
    alice_ticket=$(ockam project ticket --usage-count 1 --expires-in 100h)

    if [[ -n "$OCKAM_VERSION" ]]; then
        export OCKAM_VERSION="v${OCKAM_VERSION}";
    fi

    echo; pushd administrator_bob; ./run.sh "$bob_ticket"; popd
    echo; pushd user_alice; ./run.sh "$alice_ticket"; popd
}


# Cleanup after the example - `./run.sh cleanup`
# Remove all resources that were created in AWS.
cleanup() {
    pushd administrator_bob; ./run.sh cleanup; popd
    pushd user_alice; ./run.sh cleanup; popd
}

# Check if Ockam Command is already installed and available in path.
# If it's not, then install it.
if ! type ockam &>/dev/null; then
    curl --proto '=https' --tlsv1.2 -sSfL https://install.command.ockam.io | bash
    source "$HOME/.ockam/env"
fi

# Check that tools we we need installed.
for c in curl aws; do
    if ! type "$c" &>/dev/null; then echo "ERROR: Please install: $c" && exit 1; fi
done

# Check if the first argument is "cleanup"
# If it is, call the cleanup function. If not, call the run function.
if [ "$1" = "cleanup" ]; then
    cleanup;
else
    run;
fi
