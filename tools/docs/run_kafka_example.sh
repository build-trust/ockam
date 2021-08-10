#!/usr/bin/env bash

if [ -z $OCKAM_HOME ]; then
    echo "Please set OCKAM_HOME to the repo root"
    exit -1
fi

export TOOLS_DIR="$OCKAM_HOME/tools/docs"
export KAFKA="$OCKAM_HOME/examples/rust/ockam_kafka/"
export KAFKA_SCRIPT="$TOOLS_DIR/example_runner/kafka.ron"

if [ -z $(which example_runner) ]; then
    echo "Building example_runner utility"
    pushd "$TOOLS_DIR/example_runner" &>/dev/null
    cargo -q install --path .
    popd &>/dev/null
fi


pushd $KAFKA
example_runner $KAFKA_SCRIPT
popd
