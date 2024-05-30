#!/usr/bin/env bash
set -ex

# This script is used as an entrypoint to a docker container built using kafka_ockam.dockerfile.
# Create an Ockam node from this `ockam.yaml` descriptor file.
cat <<EOF > ./ockam.yaml
name: kafka_inlet_node
ticket: ${ENROLLMENT_TICKET}

# Declare Kafka Inlet, bind on localhost.
# The destination is the kafka_inlet node, reachable through
# the project relay named kafka.
kafka-inlet:
  from: 127.0.0.1:9092
  to: /project/default/service/forward_to_kafka/secure/api
  consumer-relay: /project/default
EOF

# optional, reduces warnings in the log
# optional, reduces warnings in the log and order the output
if echo "$@" | grep kafka-console-producer.sh; then
  sleep 17;
else
  sleep 10;
fi;

ockam node create ./ockam.yaml

# Execute the command specified in 'docker-compose.yml'
bash "$@"
