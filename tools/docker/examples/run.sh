#!/usr/bin/env bash

if [ -z "$1" ]
then
  echo "Missing Ockam Hub address argument"
  exit 0
fi

docker run -e OCKAM_HUB="$1" -e REV="$2" -it ockam-example-runner:latest

