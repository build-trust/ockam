#!/usr/bin/env bash
set -e

# This script, `./run.sh ...` is invoked on a developer’s work machine.
#
# This hands-on example uses Ockam to create an end-to-end encrypted portal to Aiven Kafka Cloud.
# We connect a kafka client in one virtual private network with a Kafka event streamer
# in another virtual private network.
#
# The example uses docker and docker compose to create these virtual networks.
#
# You can read a detailed walkthrough of this example at:
# https://docs.ockam.io/portals/kafka/aiven/docker

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

  # Creates a Kafka cluster and it's API key and secret using Aiven CLI
  random_char=$(echo $RANDOM | md5sum | head -c 10)

  service_name="ockam-demo-${random_char}"
  avn service create "$service_name" -t kafka -p startup-2 --cloud aws-us-east-1

  # Wait for service to be running
  avn service wait "$service_name"

  # avn service update "$service_name" -c kafka.auto_create_topics_enable=true
  avn service update "$service_name" -c kafka_authentication_methods.sasl=true

  service_details=$(avn service get "$service_name"  --json)
  kafka_username=$(jq -r '.users[0].username' <<< $service_details)
  kafka_password=$(jq -r '.users[0].password' <<< $service_details)

  components=$(jq '.components' <<< $service_details)
  components_len=$(jq '.|length' <<< $components)

  for ((c=0; c<$components_len; c++)); do
    authentication_method=$(jq -r ".[$c].kafka_authentication_method" <<< $components)
    if [[ "$authentication_method" != "sasl" ]]; then
      continue
    fi

    bootstrap_address="$(jq -r ".[$c].host" <<< $components):$(jq -r ".[$c].port" <<< $components)"
  done

  echo "Kafka Service ready $bootstrap_address"

  # Creates an End-to-End encrypted relay for our Aiven Kafka service.
  ockam project addon configure confluent --bootstrap-server "$bootstrap_address"

  # Create an enrollment ticket to enroll the identity used by an ockam node that will run
  # adjacent to the Kafka client app in application_team's network.
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
  #
  # Our Aiven authentication details (api key and secrets) are also passed as environment
  # variables to be used by the application team.
  echo; pushd application_team; PRODUCER_ENROLLMENT_TICKET="$application_team_producer_ticket" CONSUMER_ENROLLMENT_TICKET="$application_team_consumer_ticket" \
    KAFKA_CLUSTER_API_KEY="$kafka_username" KAFKA_CLUSTER_API_SECRET="$kafka_password" \
    docker compose up; popd
}

# Cleanup after the example - `./run.sh cleanup`
# Remove all containers and images pulled or created by docker compose.
cleanup() {
  pushd application_team; docker compose down --rmi all --remove-orphans; popd

  # Remove all Aiven Kafka clusters,
  clusters=$(avn service list --json)
  clusters_len=$(jq '.|length' <<< $clusters)

  for ((c=0; c<$clusters_len; c++)); do
    cluster_name=$(jq -r ".[${c}].service_name" <<< "$clusters")

    if [[ "$cluster_name" == *"ockam-demo"* ]]; then
      avn service terminate "$cluster_name" --force
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
for c in avn docker curl jq; do
  if ! type "$c" &>/dev/null; then echo "ERROR: Please install: $c" && exit 1; fi
done

# Check if the first argument is "cleanup"
# If it is, call the cleanup function. If not, call the run function.
if [[ "$1" == "cleanup" ]]; then cleanup; else run; fi
