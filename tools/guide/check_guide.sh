#!/usr/bin/env bash

if [ -z $OCKAM_HOME ]; then
    echo "Please set OCKAM_HOME to the repo root"
    exit -1
fi

export GUIDES_DIR="$OCKAM_HOME/documentation/guides/rust/get-started"
export EXAMPLES_DIR="$OCKAM_HOME/examples/rust/get_started/examples"
export TOOLS_DIR="$OCKAM_HOME/tools/guide"

if [ -z $(which example_blocks) ]; then
    echo "Building example_blocks utility"
    pushd "$TOOLS_DIR/example_blocks" &>/dev/null
    cargo -q install --path .
    popd &>/dev/null
fi


for page in $(find $GUIDES_DIR -name README.md); do
    $TOOLS_DIR/verify_md.sh $page || exit 1
done
