#!/bin/bash
set -ex

cat <<EOF > ./ockam.yaml
name: kafka_inlet_node
ticket: ${ENROLLMENT_TICKET}

kafka-inlet:
  from: 127.0.0.1:9092
  disable-content-encryption: true
  avoid-publishing: true
  allow: snowflake-kafka-outlet
  to: /project/default/service/forward_to_kafka/secure/api
EOF

ockam node create --foreground ./ockam.yaml
