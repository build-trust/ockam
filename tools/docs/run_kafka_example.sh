#!/usr/bin/env bash

if [ -z $OCKAM_HOME ]; then
    echo "Please set OCKAM_HOME to the repo root"
    exit -1
fi

export TOOLS_DIR="$OCKAM_HOME/tools/docs"
export KAFKA="$OCKAM_HOME/examples/rust/ockam_kafka/"
export KAFKA_SCRIPT="$TOOLS_DIR/example_runner/kafka.ron"

pushd $KAFKA
cargo run -p example_runner -- $KAFKA_SCRIPT
popd
