#!/bin/bash
set -ex

cat <<EOF > ./ockam.yaml
name: kafka_inlet_node
ticket: ${INLET_TICKET}

kafka-inlet:
  from: 127.0.0.1:9092
  disable-content-encryption: true
  allow: snowflake-kafka-outlet
  to: /project/default/service/forward_to_kafka/secure/api
EOF

export OCKAM_DISABLE_UPGRADE_CHECK=true
export OCKAM_OPENTELEMETRY_EXPORT=false

ockam node create -vv --foreground ./ockam.yaml
