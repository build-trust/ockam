#!/usr/bin/env bash

if [ -z $OCKAM_HOME ]; then
    echo "Please set OCKAM_HOME to the repo root"
    exit -1
fi

export TOOLS_DIR="$OCKAM_HOME/tools/docs"

export GET_STARTED_CODE="$OCKAM_HOME/examples/rust/get_started/"
export GET_STARTED_SCRIPT="$TOOLS_DIR/example_runner/get_started.ron"

export TCP_INLET_AND_OUTLET_CODE="$OCKAM_HOME/examples/rust/tcp_inlet_and_outlet/"
export TCP_INLET_AND_OUTLET_SCRIPT="$TOOLS_DIR/example_runner/tcp_inlet_and_outlet.ron"

# Run example_runner for each case
function do_run {
    pushd $1 &>/dev/null
    echo "============================================================"
    echo "Running $2"
    cargo run -p example_runner -- $2
    popd &>/dev/null
}
do_run $GET_STARTED_CODE $GET_STARTED_SCRIPT
do_run $TCP_INLET_AND_OUTLET_CODE $TCP_INLET_AND_OUTLET_SCRIPT
