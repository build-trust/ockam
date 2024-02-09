#!/usr/bin/env bash
set -e

# This script, `./run.sh ...` is invoked on a developer’s work machine.
#
# In this hands-on example we use Ockam to create an encrypted portal to postgres.
# We connect a nodejs app in one virtual private network with a postgres database
# in another virtual private network.
#
# We use docker and docker compose to create these virtual networks.
#
# You can read a detailed walkthough of this example at:
# https://docs.ockam.io/portals/databases/postgres/docker

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
    # adjacent to the postgres server in bank_corp's network.
    #
    # The identity that enrolls with the generated ticket will be given a project membership
    # credential in which the project membership authority will cryptographically attest to the
    # specified attributes - postgres-outlet=true.
    #
    # The identity will also allowed to create a relay in the project at the address `postgres`.
    ticket=$(ockam project ticket --usage-count 1 --expires-in 10m \
        --attribute "postgres-outlet=true" --relay postgres)

    # Invoke `docker-compose up` in the directory that has bank_corp's configuration.
    # Pass the above enrollment ticket as an environment variable.
    #
    # Read bank_corp/docker-compose.yml to understand the parts that are provisioned
    # in bank_corp's virtual private network.
    echo; pushd bank_corp; ENROLLMENT_TICKET="$ticket" docker-compose up -d; popd

    # Wait 30 seconds, to give the outlet and relay some time to be ready.
    sleep 30

    # Create an enrollment ticket to enroll the identity used by an ockam node that will run
    # adjacent to the postgres client app in analysis_corp's network.
    #
    # The identity that enrolls with the generated ticket will be given a project membership
    # credential in which the project membership authority will cryptographically attest to the
    # specified attributes - postgres-inlet=true.
    ticket=$(ockam project ticket --usage-count 1 --expires-in 10m \
        --attribute "postgres-inlet=true")

    # Invoke `docker-compose up` in the directory that has analysis_corp's configuration.
    # Pass the above enrollment ticket as an environment variable.
    #
    # Read analysis_corp/docker-compose.yml to understand the parts that are provisioned
    # in analysis_corp's virtual private network.
    echo; pushd analysis_corp; ENROLLMENT_TICKET="$ticket" docker-compose up; popd
}

# Cleanup after the example - `./run.sh cleanup`
# Remove all containers and images pulled or created by docker compose.
cleanup() {
    for d in bank_corp analysis_corp; do
        pushd "$d"; docker-compose down --rmi all --remove-orphans; popd
    done
}

# Check if Ockam Command is already installed and available in path.
# If it's not, then install it.
type ockam >/dev/null 2>&1 || curl --proto '=https' --tlsv1.2 -sSfL https://install.command.ockam.io | bash
source "$HOME/.ockam/env"

# Check that tools we we need installed.
for c in docker docker-compose curl; do
    command -v "$c" >/dev/null 2>&1 || { echo "ERROR: $c is not installed." && exit 1; }
done

# Check if the first argument is "cleanup"
# If it is, call the cleanup function. If not, call the run function.
[[ "$1" == "cleanup" ]] && cleanup || run
