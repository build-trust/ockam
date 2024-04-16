#!/usr/bin/env bash
set -e

# This script is used as an entrypoint to a docker container built using kafka_client.dockerfile.
# Create an Ockam node from this `ockam.yaml` descriptor file.
cat <<EOF > ./ockam.yaml
name: redpanda_inlet_node
ticket: ${ENROLLMENT_TICKET}

# Declare Kafka Inlet, bind on localhost.
# The destination is the 'redpanda_outlet_node' node, reachable through
# the project relay named 'redpanda'.
kafka-inlet:
  from: 127.0.0.1:9092
  to: /project/default/service/forward_to_redpanda/secure/api
EOF

# optional, reduces warnings in the log and order the output
if echo "$@" | grep kafka-console-producer.sh; then
  sleep 17;
else
  sleep 10;
fi;

set -x
ockam node create ./ockam.yaml
set +x

# Execute the command specified in 'docker-compose.yml'
bash "$@"
