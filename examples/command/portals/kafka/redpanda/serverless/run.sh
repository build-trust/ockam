#!/usr/bin/env bash
set -e

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
    check_parameters

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
    # adjacent to the Redpanda client app in application_team's network.
    #
    # The identity that enrolls with the generated ticket will be given a cryptographically
    # attestest project membership credential issue by the membership authority.
    application_team_consumer_ticket=$(ockam project ticket --usage-count 1 --expires-in 10m --relay '*')
    application_team_producer_ticket=$(ockam project ticket --usage-count 1 --expires-in 10m)

    redpanda_client_config=$(generate_config)

    # Invoke `docker-compose up` in the directory that has application_team's configuration.
    # Pass the above enrollment ticket as an environment variable.
    #
    # Read application_team/docker-compose.yml to understand the parts that are provisioned
    # in application_team's virtual private network.
    echo; pushd application_team; CONFIG="$redpanda_client_config" PRODUCER_ENROLLMENT_TICKET="$application_team_producer_ticket" CONSUMER_ENROLLMENT_TICKET="$application_team_consumer_ticket" docker compose up; popd
}

# Cleanup after the example - `./run.sh cleanup`
# Remove all containers and images pulled or created by docker compose.
cleanup() {
    pushd application_team; docker compose down --rmi all --remove-orphans; popd
}

# Check that the required environment variables are set.
check_parameters() {
    if [ -z "${REDPANDA_USERNAME}" ]; then
        echo "Please set the REDPANDA_USERNAME environment variable to your Redpanda username."
        exit 1
    fi

    if [ -z "${REDPANDA_PASSWORD}" ]; then
        echo "Please set the REDPANDA_PASSWORD environment variable to your Redpanda password."
        exit 1
    fi

    if [ -z "${REDPANDA_ADDRESS}" ]; then
        echo "Please set the REDPANDA_ADDRESS environment variable to the address of your Redpanda cluster."
        exit 1
    fi

    if [ -z "${REDPANDA_SASL_MECHANISM}" ]; then
        echo "Please set the REDPANDA_SASL_MECHANISM environment variable to the SASL mechanism of your Redpanda cluster."
        exit 1
    fi
}

# Generate a kafka client configuration for Redpanda Serverless.
generate_config() {
    cat <<EOF
security.protocol=SASL_PLAINTEXT
sasl.mechanism=${REDPANDA_SASL_MECHANISM}
sasl.jaas.config=org.apache.kafka.common.security.scram.ScramLoginModule required \
  username="${REDPANDA_USERNAME}" password="${REDPANDA_PASSWORD}";
request.timeout.ms=30000
EOF
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
