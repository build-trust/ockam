#!/usr/bin/env bash
set -ex

# This script, `./run.sh ...` is invoked on a developer’s work machine.
#
# This hands-on example uses Ockam to create an end-to-end encrypted portal to a private API. We connect
# a nodejs app in one virtual private network with a private API in another virtual private network.
#
# The example uses docker and docker compose to create these virtual networks.
#
# You can read a detailed walkthough of this example at:
# https://docs.ockam.io/portals/apis/nodejs/docker

run() {
    if [ "$1" = "with_model" ]; then download_model; fi

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
    # adjacent to the API serving HTTP server in ai_corp's network.
    #
    # The identity that enrolls with the generated ticket will be given a project membership
    # credential in which the project membership authority will cryptographically attest to the
    # specified attributes - ai-outlet=true.
    #
    # The identity will also allowed to create a relay in the project at the address `ai`.
    ai_corp_ticket=$(ockam project ticket --usage-count 1 --expires-in 10m \
        --attribute "ai-outlet=true" --relay ai)

    # Create an enrollment ticket to enroll the identity used by an ockam node that will run
    # adjacent to the API client app in health_corp's network.
    #
    # The identity that enrolls with the generated ticket will be given a project membership
    # credential in which the project membership authority will cryptographically attest to the
    # specified attributes - ai-inlet=true.
    health_corp_ticket=$(ockam project ticket --usage-count 1 --expires-in 10m \
        --attribute "ai-inlet=true")

    # Invoke `docker-compose up` in the directory that has ai_corp's configuration.
    # Pass the above enrollment ticket as an environment variable.
    #
    # Read ai_corp/docker-compose.yml to understand the parts that are provisioned
    # in ai_corp's virtual private network.
    echo; pushd ai_corp; ENROLLMENT_TICKET="$ai_corp_ticket" docker-compose up -d; popd

    # Invoke `docker-compose up` in the directory that has health_corp's configuration.
    # Pass the above enrollment ticket as an environment variable.
    #
    # Read health_corp/docker-compose.yml to understand the parts that are provisioned
    # in health_corp's virtual private network.
    echo; pushd health_corp; ENROLLMENT_TICKET="$health_corp_ticket" docker-compose up; popd
}

download_model() {
    mkdir -p ai_corp/models
    pushd ai_corp/models
    if [ ! -f "capybarahermes-2.5-mistral-7b.Q6_K.gguf" ]; then
        curl --proto '=https' --tlsv1.2 -sSfL \
            https://huggingface.co/TheBloke/CapybaraHermes-2.5-Mistral-7B-GGUF/resolve/main/capybarahermes-2.5-mistral-7b.Q6_K.gguf
    fi
    popd
}

# Cleanup after the example - `./run.sh cleanup`
# Remove all containers and images pulled or created by docker compose.
cleanup() {
    pushd ai_corp; docker-compose down --rmi all --remove-orphans; popd
    pushd health_corp; docker-compose down --rmi all --remove-orphans; popd
}

# Check if Ockam Command is already installed and available in path.
# If it's not, then install it.
if ! type ockam &>/dev/null; then
    curl --proto '=https' --tlsv1.2 -sSfL https://install.command.ockam.io | bash
    source "$HOME/.ockam/env"
fi

# Check that tools we we need installed.
for c in docker docker-compose curl; do
    if ! type "$c" &>/dev/null; then echo "ERROR: Please install: $c" && exit 1; fi
done

# Check if the first argument is "cleanup"
# If it is, call the cleanup function. If not, call the run function.
if [ "$1" = "cleanup" ]; then cleanup; else run $@; fi
