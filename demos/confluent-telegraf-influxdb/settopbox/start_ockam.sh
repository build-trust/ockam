#!/bin/sh
NAME="settopbox"

ockam identity create "$NAME"_identity
ockam project enroll $PRODUCER_TOKEN --identity "$NAME"_identity
ockam node create "$NAME"_node --identity "$NAME"_identity
ockam kafka-producer create --at "$NAME"_node
sleep 60
