#!/bin/bash
set -ex

# This script is used as an entrypoint to a docker container built using kafka_ockam.dockerfile.
# Create an Ockam node from this `ockam.yaml` descriptor file.
kafka_host=$(dig +short kafka)

cat <<EOF > ./ockam.yaml
name: kafka_outlet_node
ticket: ${ENROLLMENT_TICKET}
# This node will be reachable in the project using the address 'forward_to_kafka'.
relay: kafka
# Declare a Kafka Outlet, with a local destination.
kafka-outlet:
  bootstrap-server: ${kafka_host}:9092
# Declare a local Kafka Inlet, bind to localhost,
# pointing to this very node. Kafka messages will
# be transparently encrypted.
kafka-inlet:
  from: 127.0.0.1:59092
  to: /secure/api
EOF

ockam node create -f ./ockam.yaml
