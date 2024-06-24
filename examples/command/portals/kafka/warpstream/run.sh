#!/usr/bin/env bash
set -e

# This script, `./run.sh ...` is invoked on a developer’s work machine.
#
# This hands-on example uses Ockam to create an end-to-end encrypted portal to Warpstream.
# We connect a kafka client in one virtual private network with a Warpstream event streamer
# in another virtual private network.
#
# The example uses docker and docker compose to create these virtual networks.
#
# You can read a detailed walkthrough of this example at:
# https://docs.ockam.io/portals/kafka/warpstream/docker

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

    # Create a Cluster in Warpstream
    echo "$WARPSTREAM_API_KEY $OCKAM_VERSION"
    if [[ -z $WARPSTREAM_API_KEY ]]; then echo "ERROR: Please provide your Warpstream API key as an environment variable 'WARPSTREAM_API_KEY'" && exit 1; fi;

    cluster_detail=$(curl --silent --show-error --fail https://api.prod.us-east-1.warpstream.com/api/v1/create_virtual_cluster \
        -H "warpstream-api-key: $WARPSTREAM_API_KEY" \
        -H 'Content-Type: application/json' \
        -d '{"virtual_cluster_name": "ockam_demo", "virtual_cluster_type": "serverless", "virtual_cluster_region": "us-east-1", "virtual_cluster_cloud_provider": "aws"}')

    request_body=$(docker run -i ghcr.io/jqlang/jq '. | {"credentials_name": "ockam_demo", "is_cluster_superuser":true, "agent_pool_id": .agent_pool_id, "virtual_cluster_id": .virtual_cluster_id}' <<< $cluster_detail)

    cluster_credential=$(curl --silent --show-error --fail https://api.prod.us-east-1.warpstream.com/api/v1/create_virtual_cluster_credentials \
        -H "warpstream-api-key: $WARPSTREAM_API_KEY" \
        -H 'Content-Type: application/json' \
        -d "$request_body")

    bootstrap_username=$(docker run -i ghcr.io/jqlang/jq -r '.username' <<< $cluster_credential)
    bootstrap_password=$(docker run -i ghcr.io/jqlang/jq -r '.password' <<< $cluster_credential)

    # Creates an End-to-End encrypted relay for our Wrapstream Kafka service.
    ockam project addon configure confluent --bootstrap-server serverless.warpstream.com:9092

    # Create an enrollment ticket to enroll the identity used by an ockam node that will run
    # adjacent to the Warpstream client app in application_team's network.
    #
    # The identity that enrolls with the generated ticket will be given a cryptographically
    # attestest project membership credential issue by the membership authority.
    application_team_consumer_ticket=$(ockam project ticket --usage-count 1 --expires-in 10m --relay '*')
    application_team_producer_ticket=$(ockam project ticket --usage-count 1 --expires-in 10m --relay '*')

    # Invoke `docker-compose up` in the directory that has application_team's configuration.
    # Pass the above enrollment ticket as an environment variable.
    #
    # Read application_team/docker-compose.yml to understand the parts that are provisioned
    # in application_team's virtual private network.
    echo; pushd application_team; PRODUCER_ENROLLMENT_TICKET="$application_team_producer_ticket" CONSUMER_ENROLLMENT_TICKET="$application_team_consumer_ticket" \
        KAFKA_CLUSTER_API_KEY="$bootstrap_username" KAFKA_CLUSTER_API_SECRET="$bootstrap_password" \
        docker compose up; popd
}

# Cleanup after the example - `./run.sh cleanup`
# Remove all containers and images pulled or created by docker compose.
cleanup() {
    if [[ -z $WARPSTREAM_API_KEY ]]; then echo "ERROR: Please provide your Warpstream API key" && exit 1; fi;
    pushd application_team; docker compose down --rmi all --remove-orphans; popd

    clusters=$(curl --silent --show-error --fail https://api.prod.us-east-1.warpstream.com/api/v1/list_virtual_clusters \
        -H "warpstream-api-key: $WARPSTREAM_API_KEY" \
        -H 'Content-Type: application/json')
    cluster_len=$(docker run -i ghcr.io/jqlang/jq '.virtual_clusters|length' <<< $clusters)
    echo "$cluster_len"

    for ((c=0; c<$cluster_len; c++)); do
        cluster_name=$(docker run -i ghcr.io/jqlang/jq -r ".virtual_clusters.[${c}].name" <<< $clusters)
        cluster_id=$(docker run -i ghcr.io/jqlang/jq -r ".virtual_clusters.[${c}].id" <<< $clusters)

        if [[ "$cluster_name" == *"ockam_demo"* ]]; then
            echo "Deleting cluster wrapstream cluster $cluster_name"

            request_body=$(docker run -i ghcr.io/jqlang/jq "{\"virtual_cluster_id\": \"$cluster_id\", \"virtual_cluster_name\": \"$cluster_name\"}" <<< "{}")

            curl --silent --show-error --fail https://api.prod.us-east-1.warpstream.com/api/v1/delete_virtual_cluster \
                -H "warpstream-api-key: $WARPSTREAM_API_KEY" \
                -H 'Content-Type: application/json' \
                -d "$request_body"
        fi
    done
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
if [ "$1" = "cleanup" ]; then
    export WARPSTREAM_API_KEY="$2"
    cleanup;
else
    export WARPSTREAM_API_KEY="$1"
    run;
fi
