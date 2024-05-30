#!/usr/bin/env bash
set -e

cat >kafka.config <<EOF
request.timeout.ms=30000
security.protocol=SASL_PLAINTEXT
sasl.mechanism=SCRAM-SHA-256
sasl.jaas.config=org.apache.kafka.common.security.scram.ScramLoginModule required \
username="myKafkaUser" \
password="myPassword1.";
EOF

# This script is used as an entrypoint to a docker container built using kafka_client.dockerfile.
# Create an Ockam node from this `ockam.yaml` descriptor file.
cat <<EOF > ./ockam.yaml
name: instaclustr_inlet_node
ticket: ${ENROLLMENT_TICKET}

# Declare Instaclustr Inlet, bind on localhost.
# The destination is the 'instaclustr_outlet_node' node, reachable through
# the project relay named 'instaclustr'.
kafka-inlet:
  from: 127.0.0.1:9092
  to: /project/default/service/forward_to_instaclustr/secure/api
  consumer-relay: /project/default
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
