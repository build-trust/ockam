#!/usr/bin/env bash
set -e
set -m
set -x

ockam identity create 
ockam project enroll /mnt/ticket
ockam node create
ockam kafka-consumer create

kafka-console-consumer.sh --topic ockam-demo --bootstrap-server 127.0.0.1:4000 