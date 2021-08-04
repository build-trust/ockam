#!/usr/bin/env bash

if [ -z $OCKAM_HOME ]; then
    echo "Please set OCKAM_HOME to the repo root"
    exit -1
fi

export TOOLS_DIR="$OCKAM_HOME/tools/docs"
export GET_STARTED="$OCKAM_HOME/examples/rust/get_started/"
export GET_STARTED_SCRIPT="$TOOLS_DIR/example_runner/get_started.ron"

if [ -z $(which example_runner) ]; then
    echo "Building example_runner utility"
    pushd "$TOOLS_DIR/example_runner" &>/dev/null
    cargo -q install --path .
    popd &>/dev/null
fi


pushd $GET_STARTED
echo $GET_STARTED_SCRIPT
example_runner $GET_STARTED_SCRIPT
popd
