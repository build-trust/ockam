#!/usr/bin/env bash
set -e

# This script, `./run.sh ...` is invoked on a developer’s work machine.
#
# This hands-on example uses Ockam to create an end-to-end encrypted portal to Instaclustr.
# We connect a kafka client in one virtual private network with a Instaclustr event streamer
# in another virtual private network.
#
# The example uses docker and docker compose to create these virtual networks.
#
# You can read a detailed walkthrough of this example at:
# https://docs.ockam.io/portals/kafka/instaclustr/docker

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
    #
    # Create a Kafka cluster in Instaclustr
    #
    echo "============================================"
    echo "This example requires Instaclustr username and API key to continue"
    echo " - Create an account if you don't have already at https://www.instaclustr.com/platform/managed-apache-kafka/"
    echo " - Upon signing in, Account API keys can be created from the console by going to gear icon to the top right > Account Settings > API Keys"
    echo " - Create a Provisioning API key to use for this demo"
    echo "Press Ctrl+c to exit"
    echo "============================================"
    instaclustr_auth
    bootstrap_server=$(./cluster_manager.sh | tee /dev/tty | grep 'BOOTSTRAP_SERVER:' | cut -d':' -f2)

    if ! [[ $bootstrap_server =~ ^[0-9]+\.[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
        echo "ERROR: Creating kafka cluster in instaclustr failed"
        exit 1
    fi
    # Create an enrollment ticket to enroll the identity used by an ockam node that will run
    # adjacent to the Instaclustr server in instaclustr_operator's network.
    #
    # The identity that enrolls with the generated ticket will be given a cryptographically
    # attestest project membership credential issue by the membership authority.
    #
    # The identity will also allowed to create a relay in the project at the address `instaclustr`.
    instaclustr_operator_ticket=$(ockam project ticket --usage-count 1 --expires-in 10m --relay instaclustr)

    # Create an enrollment ticket to enroll the identity used by an ockam node that will run
    # adjacent to the Instaclustr client app in application_team's network.
    #
    # The identity that enrolls with the generated ticket will be given a cryptographically
    # attestest project membership credential issue by the membership authority.
    application_team_consumer_ticket=$(ockam project ticket --usage-count 1 --expires-in 10m --relay '*')
    application_team_producer_ticket=$(ockam project ticket --usage-count 1 --expires-in 10m --relay '*')

    # Invoke `docker-compose up` in the directory that has instaclustr_operator's configuration.
    # Pass the above enrollment ticket as an environment variable.
    #
    # Read instaclustr_operator/docker-compose.yml to understand the parts that are provisioned
    # in instaclustr_operator's virtual private network.
    echo; pushd instaclustr_operator; ENROLLMENT_TICKET="$instaclustr_operator_ticket" BOOTSTRAPSERVER="$bootstrap_server" docker compose up -d; popd

    # Invoke `docker-compose up` in the directory that has application_team's configuration.
    # Pass the above enrollment ticket as an environment variable.
    #
    # Read application_team/docker-compose.yml to understand the parts that are provisioned
    # in application_team's virtual private network.
    echo; pushd application_team; PRODUCER_ENROLLMENT_TICKET="$application_team_producer_ticket" CONSUMER_ENROLLMENT_TICKET="$application_team_consumer_ticket" docker compose up; popd
}

# Cleanup after the example - `./run.sh cleanup`
# Remove all containers and images pulled or created by docker compose.
cleanup() {
    instaclustr_auth
    ./cluster_manager.sh cleanup
    pushd instaclustr_operator; docker compose down --rmi all --remove-orphans; popd
    pushd application_team; docker compose down --rmi all --remove-orphans; popd
}

instaclustr_auth() {
    #
    # Set Instaclustr credentials to create and delete a free trial kafka cluster

    if [[ -z "${INSTACLUSTR_USER_NAME}" || -z "${INSTACLUSTR_API_KEY}" ]]; then
        echo -n "Enter Instaclustr username: "
        read instaclustr_username
        echo -n "Enter Instaclustr API key: "
        read -s instaclustr_api_key
        echo

        # Export the variables as environment variables
        export INSTACLUSTR_USER_NAME="${instaclustr_username}"
        export INSTACLUSTR_API_KEY="${instaclustr_api_key}"
    else
        echo "Using existing INSTACLUSTR_USER_NAME and INSTACLUSTR_API_KEY environment variables"
    fi
}
# Check if Ockam Command is already installed and available in path.
# If it's not, then install it.
if ! type ockam &>/dev/null; then
    curl --proto '=https' --tlsv1.2 -sSfL https://install.command.ockam.io | bash
    source "$HOME/.ockam/env"
fi

# Check that tools we we need installed.
for c in docker curl jq; do
    if ! type "$c" &>/dev/null; then echo "ERROR: Please install: $c" && exit 1; fi
done

# Check if the first argument is "cleanup"
# If it is, call the cleanup function. If not, call the run function.
if [ "$1" = "cleanup" ]; then cleanup; else run; fi
