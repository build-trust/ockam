#!/usr/bin/env bash
set -ex

# This script is used as an entrypoint to a docker container built using ../ockam.dockerfile.
# Create an Ockam node from this `ockam.yaml` descriptor file.
cat <<EOF > ./ockam.yaml
name: redpanda_outlet_node
ticket: ${ENROLLMENT_TICKET}

# This node will be reachable in the project
# using the address 'forward_to_redpanda'.
relay: redpanda

# Declare a Kafka Outlet, with a local destination.
kafka-outlet:
  bootstrap-server: redpanda:9092
  allow: '(or (= subject.kafka-consumer "true") (= subject.kafka-producer "true"))'
EOF

# Create the Ockam node in foreground mode.
ockam node create -f ./ockam.yaml
