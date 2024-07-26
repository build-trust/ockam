#!/bin/bash
set -ex

# Don't check for the latest Ockam version
export OCKAM_DISABLE_UPGRADE_CHECK=true

# Don't export traces and log messages
export OCKAM_OPENTELEMETRY_EXPORT=false

# print the environment to double-check it
echo "environment variables"
env

# start the node
cat <<EOF > ./ockam.yaml
name: kafka_inlet_node
ticket: ${CONSUMER_TICKET}

kafka-inlet:
  from: ${KAFKA_BOOTSTRAP_SERVERS}
  disable-content-encryption: true
  allow: snowflake-kafka-outlet
  to: /project/default/service/forward_to_kafka/secure/api
EOF

ockam node create -vv --foreground ./ockam.yaml &

python3 service.py
