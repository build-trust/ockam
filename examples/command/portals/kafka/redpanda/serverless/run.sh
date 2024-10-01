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

redpanda() {
  if test -f application_team/volumes/config/kafka.config; then
    echo ""
    echo "Redpanda config already exists."
  else
    echo ""
    echo "Create a new Redpanda Serverless cluster, if you haven't already, at https://cloud.redpanda.com/clusters"
    echo "then on the 'Overview' page open the 'Kafka API' tab and click 'view credentials'."
    echo ""
    read -p "What is the username: " username
    read -p "What is the password: " -s password
    echo ""
    read -p "What is the Bootstrap server URL: " bootstrap_uri
    echo ""

    ockam project addon configure redpanda \
      --bootstrap-server $bootstrap_uri

    mkdir -p application_team/volumes/config
    cat >application_team/volumes/config/kafka.config <<EOF
request.timeout.ms=30000
sasl.mechanism=SCRAM-SHA-256
security.protocol=SASL_SSL
sasl.jaas.config=org.apache.kafka.common.security.scram.ScramLoginModule required \
        username="$username" \
        password="$password";

producer.sasl.mechanism=SCRAM-SHA-256
producer.security.protocol=SASL_SSL
producer.sasl.jaas.config=org.apache.kafka.common.security.scram.ScramLoginModule  required \
        username="$username" \
        password="$password";

consumer.sasl.mechanism=SCRAM-SHA-256
consumer.security.protocol=SASL_SSL
consumer.sasl.jaas.config=org.apache.kafka.common.security.scram.ScramLoginModule  required \
        username="$username" \
        password="$password";
EOF
  fi
}
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
  redpanda

  # Create an enrollment ticket to enroll the identity used by an ockam node that will run
  # adjacent to the Redpanda client app in application_team's network.
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
  echo
  pushd application_team
  PRODUCER_ENROLLMENT_TICKET="$application_team_producer_ticket" CONSUMER_ENROLLMENT_TICKET="$application_team_consumer_ticket" docker compose up
  popd
}

# Cleanup after the example - `./run.sh cleanup`
# Remove all containers and images pulled or created by docker compose.
cleanup() {
  pushd application_team
  docker compose down --rmi all --remove-orphans
  popd
  rm -rf application_team/volumes/config
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
