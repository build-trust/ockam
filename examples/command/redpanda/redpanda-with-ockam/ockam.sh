#!/usr/bin/env bash
set -e
set -m
set -x

/entrypoint.sh "$@" &

ockam identity create
ockam project enroll /mnt/ticket
ockam node create --tcp-listener-address 0.0.0.0:6000
sleep 2

ockam kafka-outlet create \
  --bootstrap-server 0.0.0.0:9092

fg %1
