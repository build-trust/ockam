#!/usr/bin/env bash
set -e
set -m
set -x

sleep 10
ockam identity create
ockam project enroll /mnt/ticket
ockam node create
ockam kafka-consumer create \
  --project-route /dnsaddr/redpanda-ockam/tcp/6000/secure/api \
  --bootstrap-server 127.0.0.1:9092

# exec "$@"
kafka-console-consumer.sh --topic demo --bootstrap-server 127.0.0.1:9092
