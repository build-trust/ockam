#!/usr/bin/env bash
set -ex

# This script is used as an entrypoint to a docker container built using redpanda.dockerfile.
# Create an Ockam node from this `ockam.yaml` descriptor file.
cat <<EOF > ./ockam.yaml
name: redpanda_outlet_node
ticket: ${ENROLLMENT_TICKET}

# This node will be reachable in the project named
# 'redpanda'.
relay: redpanda

# Declare a Kafka Outlet, with a local destination.
kafka-outlet:
  bootstrap-server: 127.0.0.1:9092

# Declare a local Kafka Inlet, bind to localhost,
# pointing to this very node. Kafka messages will
# be transparently encrypted.
kafka-inlet:
  from: 127.0.0.1:59092
  to: /secure/api
EOF

# Run in the background to save few seconds.
ockam node create ./ockam.yaml &

# Send a message to the Redpanda demo topic every 10 seconds.
# The message will be encrypted and sent to the Redpanda server,
# only the consumer will be able to decrypt it.
(
  sleep 20;
  while :; do
    sleep 10;
    echo 'The example run was successful 🥳' | rpk topic produce demo \
      --brokers 127.0.0.1:59092 \
      --allow-auto-topic-creation;
  done
) &

# Start Redpanda own entrypoint.
/entrypoint.sh "$@"