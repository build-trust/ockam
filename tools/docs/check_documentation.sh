#!/usr/bin/env bash

if [ -z "$OCKAM_HOME" ]; then
  echo "Please set OCKAM_HOME to the repo root"
  exit 1
fi

# Getting Started Guide
export GUIDE_DOCS="$OCKAM_HOME/documentation/guides/rust"
export GUIDE_EXAMPLES="$OCKAM_HOME/examples/rust/get_started/examples"

# Hello Ockam ReadMe
export HELLO_DOC="$OCKAM_HOME/README.md"
export HELLO_EXAMPLE="$OCKAM_HOME/examples/rust/get_started/examples"

# Kafka
export KAFKA_DOCS="$OCKAM_HOME/documentation/use-cases/end-to-end-encryption-through-kafka"
export KAFKA_EXAMPLES="$OCKAM_HOME/examples/rust/ockam_kafka/examples"

# E2E
export E2E_DOCS="$OCKAM_HOME/documentation/use-cases/end-to-end-encryption-with-rust"
export E2E_EXAMPLES="$OCKAM_HOME/examples/rust/get_started/examples" # TODO

# documentation/use-cases/secure-remote-access-tunnels
export INLET_DOCS="$OCKAM_HOME/documentation/use-cases/secure-remote-access-tunnels"
export INLET_EXAMPLES="$OCKAM_HOME/examples/rust/tcp_inlet_and_outlet/examples"

# documentation/use-cases/end-to-end-encrypt-all-application-layer-communication
export E2EAALC_DOCS="$OCKAM_HOME/documentation/use-cases/end-to-end-encrypt-all-application-layer-communication"
export E2EAALC_EXAMPLES="$OCKAM_HOME/examples/rust/tcp_inlet_and_outlet/examples"

# documentation/use-cases/run-ockam-on-riscv
# This documentation is not using compiled code, it could drift from the current Ockam API.

# Tools home
export TOOLS_DIR="$OCKAM_HOME/tools/docs"

# Install example_blocks binary, if needed
if [ -z "$(which example_blocks)" ]; then
  echo "Building example_blocks utility"
  pushd "$TOOLS_DIR/example_blocks" &>/dev/null || exit
  cargo -q install --path .
  popd &>/dev/null || exit
fi

ERR=0

function check_directory {
  doc_dir=$1
  dir=$2
  for page in $(find "$doc_dir" -name README.md); do
    check_readme "$page" "$dir"
  done
}

function check_readme {
  page=$1
  export EXAMPLES_DIR=$2

  if ! "$TOOLS_DIR"/verify_md.sh "$page"; then
    echo "$page has outdated examples differing from $EXAMPLES_DIR"
    ERR=1
  fi
}

check_directory "$GUIDE_DOCS" "$GUIDE_EXAMPLES"
check_directory "$KAFKA_DOCS" "$KAFKA_EXAMPLES"
check_directory "$E2E_DOCS" "$E2E_EXAMPLES"
check_directory "$INLET_DOCS" "$INLET_EXAMPLES"
check_directory "$E2EAALC_DOCS" "$E2EAALC_EXAMPLES"

check_readme "$HELLO_DOC" "$HELLO_EXAMPLE"

if [[ $ERR -eq 0 ]]; then
  echo "All okay"
fi
exit $ERR
