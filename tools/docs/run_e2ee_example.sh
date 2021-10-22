#!/usr/bin/env bash

if [ -z $OCKAM_HOME ]; then
    echo "Please set OCKAM_HOME to the repo root"
    exit -1
fi

export TOOLS_DIR="$OCKAM_HOME/tools/docs"
export E2EE="$OCKAM_HOME/examples/rust/tcp_inlet_and_outlet/"
export E2EE_SCRIPT="$TOOLS_DIR/example_runner/e2ee.ron"

if [ -z $(which example_runner) ]; then
    echo "Building example_runner utility"
    pushd "$TOOLS_DIR/example_runner" &>/dev/null
    cargo -q install --path .
    popd &>/dev/null
fi


pushd $E2EE
example_runner $E2EE_SCRIPT
popd
