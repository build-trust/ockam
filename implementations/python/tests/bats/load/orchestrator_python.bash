#!/bin/bash

if [[ -z $CLUSTER_ID ]]; then
  export CLUSTER_ID=$($OCKAM cluster show)
fi
if [[ -z $DEFAULT_HTTP_PORT ]]; then
  export DEFAULT_HTTP_PORT=32100
fi

function orchestrator_python_setup_suite() {
  export CLUSTER=$($OCKAM cluster show --jq '.cluster')
  $OCKAM zone create --zone batszone || true
  export ZONE_NAME=batszone
}

function orchestrator_python_teardown_suite() {
  $OCKAM zone delete --all --yes || true
  rm -rf $OCKAM_HOME_BASE/.tmp
}

function public_endpoint() {
  zone_name=$1
  echo "https://${CLUSTER_ID}-${zone_name}.ai.ockam.network"
}
