#!/usr/bin/env bash
set -e

# optional, reduces warnings in the log and order the output
if echo "$@" | grep kafka-console-producer.sh; then
  sleep 17;
else
  kafka-topics.sh \
    --bootstrap-server localhost:9092 \
    --command-config /etc/kafka/kafka.config \
    --create \
    --topic demo \
    --partitions 3
  sleep 10;
fi;

set -x
ockam node create ./mnt/ockam.yaml
set +x

# Execute the command specified in 'docker-compose.yml'
bash "$@"
