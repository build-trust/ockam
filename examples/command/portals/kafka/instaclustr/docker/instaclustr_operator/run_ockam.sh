#!/usr/bin/env bash
set -ex

# This script is used as an entrypoint to a docker container built using ../ockam.dockerfile.
# Create an Ockam node from this `ockam.yaml` descriptor file.
cat <<EOF > ./ockam.yaml
name: instaclustr_outlet_node
ticket: ${ENROLLMENT_TICKET}

# This node will be reachable in the project
# using the address 'forward_to_instaclustr'.
relay: instaclustr

# Declare a Kafka Outlet, with a local destination.
kafka-outlet:
  bootstrap-server: ${INSTACLUSTER_ADDRESS}
EOF

# Create the Ockam node in foreground mode.
ockam node create -f ./ockam.yaml
