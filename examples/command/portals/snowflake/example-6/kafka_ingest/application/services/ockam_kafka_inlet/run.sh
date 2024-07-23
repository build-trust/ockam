#!/bin/bash
set -ex

# Don't check for the latest Ockam version
export OCKAM_DISABLE_UPGRADE_CHECK=true

# Don't export traces and log messages
export OCKAM_OPENTELEMETRY_EXPORT=false

# print the environment to double-check it
env

# start the node
cat <<EOF > ./ockam.yaml
name: kafka_inlet_node
ticket: ${CONSUMER_TICKET}

kafka-inlet:
  from: 127.0.0.1:9092
  disable-content-encryption: true
  allow: kafka-producer
  to: /project/default/service/forward_to_kafka/secure/api
EOF

ockam node create -vv --foreground ./ockam.yaml
