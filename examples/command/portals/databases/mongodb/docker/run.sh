#!/usr/bin/env bash
set -ex

# This script, `./run.sh ...` is invoked on a developer’s work machine.
#
# This hands-on example uses Ockam to create an end-to-end encrypted portal to MongoDB. We connect a
# nodejs app in one virtual private network with a MongoDB database in another virtual private network.
#
# The example uses docker and docker compose to create these virtual networks.
#
# You can read a detailed walkthrough of this example at:
# https://docs.ockam.io/portals/databases/mongodb/docker

run() {
    # Run `ockam enroll`.
    #
    # The enroll command creates a new vault and generates a cryptographic identity with
    # private keys stored in that vault. It then guides you to sign in to Ockam Orchestrator.
    #
    # If this is your first time signing in, the Orchestrator creates a new dedicated project
    # for you. A project offers two services: a membership authority and a relay service.
    #
    # The enroll command then asks this project’s membership authority to sign and issue
    # a credential that attests that your identifier is a member of this project. Since your
    # account in Orchestrator is the creator and hence first administrator on this new project,
    # the membership authority issues this credential. The enroll command stores the
    # credential for later use and exits.
    ockam enroll

    # Create an enrollment ticket to enroll the identity used by an ockam node that will run
    # adjacent to the MongoDB server in bank_corp's network.
    #
    # The identity that enrolls with the generated ticket will be given a project membership
    # credential in which the project membership authority will cryptographically attest to the
    # specified attributes - mongodb-outlet=true.
    #
    # The identity will also allowed to create a relay in the project at the address `mongodb`.
    bank_corp_ticket=$(ockam project ticket --usage-count 1 --expires-in 10m \
        --attribute "mongodb-outlet=true" --relay mongodb)

    # Create an enrollment ticket to enroll the identity used by an ockam node that will run
    # adjacent to the MongoDB client app in analysis_corp's network.
    #
    # The identity that enrolls with the generated ticket will be given a project membership
    # credential in which the project membership authority will cryptographically attest to the
    # specified attributes - mongodb-inlet=true.
    analysis_corp_ticket=$(ockam project ticket --usage-count 1 --expires-in 10m \
        --attribute "mongodb-inlet=true")

    # Invoke `docker-compose up` in the directory that has bank_corp's configuration.
    # Pass the above enrollment ticket as an environment variable.
    #
    # Read bank_corp/docker-compose.yml to understand the parts that are provisioned
    # in bank_corp's virtual private network.
    echo; pushd bank_corp; ENROLLMENT_TICKET="$bank_corp_ticket" docker-compose up -d; popd

    # Invoke `docker-compose up` in the directory that has analysis_corp's configuration.
    # Pass the above enrollment ticket as an environment variable.
    #
    # Read analysis_corp/docker-compose.yml to understand the parts that are provisioned
    # in analysis_corp's virtual private network.
    echo; pushd analysis_corp; ENROLLMENT_TICKET="$analysis_corp_ticket" docker-compose up; popd
}

# Cleanup after the example - `./run.sh cleanup`
# Remove all containers and images pulled or created by docker compose.
cleanup() {
    pushd bank_corp; docker-compose down --rmi all --remove-orphans; popd
    pushd analysis_corp; docker-compose down --rmi all --remove-orphans; popd
}

# Check if Ockam Command is already installed and available in path.
# If it's not, then install it.
if ! type ockam &>/dev/null && ! [[ "$1" = "cleanup" ]]; then
    curl --proto '=https' --tlsv1.2 -sSfL https://install.command.ockam.io | bash
    source "$HOME/.ockam/env"
fi

# Check that tools we we need installed.
for c in docker docker-compose curl; do
    if ! type "$c" &>/dev/null; then echo "ERROR: Please install: $c" && exit 1; fi
done

# Check if the first argument is "cleanup"
# If it is, call the cleanup function. If not, call the run function.
if [ "$1" = "cleanup" ]; then cleanup; else run; fi
