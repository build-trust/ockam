#!/usr/bin/env bash
set -ex

# This script, `./run.sh ...` is invoked on a developer’s work machine.
#
# This hands-on example uses Ockam to create an end-to-end encrypted portal to Redpanda.
# We connect a kafka client in one virtual private network with a Redpanda event streamer
# in another virtual private network.
#
# The example uses docker and docker compose to create these virtual networks.
#
# You can read a detailed walkthrough of this example at:
# https://docs.ockam.io/portals/kafka/redpanda/docker

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
    # adjacent to the Redpanda server in redpanda_operator's network.
    #
    # The identity that enrolls with the generated ticket will be given a cryptographically
    # attestest project membership credential issue by the membership authority.
    #
    # The identity will also allowed to create a relay in the project at the address `redpanda`.
    redpanda_operator_ticket=$(ockam project ticket --usage-count 1 --expires-in 10m --relay redpanda --attribute redpanda)

    # Create an enrollment ticket to enroll the identity used by an ockam node that will run
    # adjacent to the Redpanda client app in application_team's network.
    #
    # The identity that enrolls with the generated ticket will be given a cryptographically
    # attestest project membership credential issue by the membership authority.
    application_team_producer_ticket=$(ockam project ticket --usage-count 1 --expires-in 10m --attribute producer --attribute inlet)
    application_team_consumer_ticket=$(ockam project ticket --usage-count 1 --expires-in 10m --attribute consumer --attribute inlet --relay consumer)

    data_producer_ticket=$(ockam project ticket --usage-count 1 --expires-in 10m --attribute data-producer --attribute inlet)
    data_consumer_ticket=$(ockam project ticket --usage-count 1 --expires-in 10m --attribute data-consumer --attribute inlet --relay data-consumer)


    # Invoke `docker-compose up` in the directory that has redpanda_operator's configuration.
    # Pass the above enrollment ticket as an environment variable.
    #
    # Read redpanda_operator/docker-compose.yml to understand the parts that are provisioned
    # in redpanda_operator's virtual private network.
    echo; pushd redpanda_operator; ENROLLMENT_TICKET="$redpanda_operator_ticket" docker compose up -d; popd

    # Invoke `docker-compose up` in the directory that has application_team's configuration.
    # Pass the above enrollment ticket as an environment variable.
    #
    # Read application_team/docker-compose.yml to understand the parts that are provisioned
    # in application_team's virtual private network.
    echo; pushd application_team_producer; PRODUCER_ENROLLMENT_TICKET="$application_team_producer_ticket" docker compose up -d; popd

    echo; pushd application_team_consumer; CONSUMER_ENROLLMENT_TICKET="$application_team_consumer_ticket" docker compose up -d; popd

    echo; pushd data_team_producer; DATA_PRODUCER_ENROLLMENT_TICKET="$data_producer_ticket" docker compose up -d; popd

    echo; pushd data_team_consumer; DATA_CONSUMER_ENROLLMENT_TICKET="$data_consumer_ticket" docker compose up -d; popd

}

# Cleanup after the example - `./run.sh cleanup`
# Remove all containers and images pulled or created by docker compose.
cleanup() {
    pushd redpanda_operator; docker compose down --rmi all --remove-orphans; popd
    pushd application_team_producer; docker compose down --rmi all --remove-orphans; popd
    pushd application_team_consumer; docker compose down --rmi all --remove-orphans; popd
    pushd data_team_producer; docker compose down --rmi all --remove-orphans; popd
    pushd data_team_consumer; docker compose down --rmi all --remove-orphans; popd
}

# Check if Ockam Command is already installed and available in path.
# If it's not, then install it.
if ! type ockam &>/dev/null; then
    curl --proto '=https' --tlsv1.2 -sSfL https://install.command.ockam.io | bash
    source "$HOME/.ockam/env"
fi

# Check that tools we we need installed.
for c in docker curl; do
    if ! type "$c" &>/dev/null; then echo "ERROR: Please install: $c" && exit 1; fi
done

# Check if the first argument is "cleanup"
# If it is, call the cleanup function. If not, call the run function.
if [ "$1" = "cleanup" ]; then cleanup; else run; fi
