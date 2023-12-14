#!/usr/bin/sh
set -e
set -m
set -x

ockam identity create
ockam project enroll /mnt/ticket
ockam node create
ockam kafka-producer create

kafka-console-producer.sh --topic ockam-demo --bootstrap-server 127.0.0.1:5000 \
  --producer.config /etc/kafka/kafka.config
