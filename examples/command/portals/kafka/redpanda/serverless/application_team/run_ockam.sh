#!/usr/bin/env bash
set -e

# This script is used as an entrypoint to a docker container built using kafka_client.dockerfile.
# Create an Ockam node from this `ockam.yaml` descriptor file.
cat <<EOF > ./ockam.yaml
name: redpanda_inlet_node
ticket: ${ENROLLMENT_TICKET}

# Declare Kafka Inlet linked to the local Kafka Outlet while using
# the project as a relay for producer to consumer communication.
kafka-inlet:
  from: 127.0.0.1:9092
  to: self
  consumer-relay: /project/default
  allow: consumer or producer
  allow-consumer: consumer
  allow-producer: producer

kafka-outlet:
  bootstrap-server: ${REDPANDA_ADDRESS}
  tls: true
  allow: consumer or producer
EOF

# optional, reduces warnings in the log and order the output
if echo "$@" | grep kafka-console-producer.sh; then
  sleep 5;
fi;

set -x
ockam node create ./ockam.yaml
set +x

# Execute the command specified in 'docker-compose.yml'
bash "$@"
