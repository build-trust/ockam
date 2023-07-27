#!/usr/bin/env bash
set -e
if [ -z "$OCKAM_HOME" ]; then
  echo "Please set OCKAM_HOME to the repo root"
  exit 1
fi

# Hello Ockam ReadMe
export HELLO_DOC="$OCKAM_HOME/README.md"
export HELLO_EXAMPLE="$OCKAM_HOME/examples/rust/get_started/examples"

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
  for page in $(find "$doc_dir" -name "*.md"); do
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

check_readme "$HELLO_DOC" "$HELLO_EXAMPLE"

if [[ -n $CHECK_MD_FILE && -n $CHECK_MD_DIR_RUST_EXAMPLE ]]; then
  check_readme "$CHECK_MD_FILE" "$CHECK_MD_DIR_RUST_EXAMPLE"
fi

if [[ -n $CHECK_MD_DIR && -n $CHECK_MD_DIR_RUST_EXAMPLE ]]; then
  check_directory "$CHECK_MD_DIR" "$CHECK_MD_DIR_RUST_EXAMPLE"
fi

if [[ $ERR -eq 0 ]]; then
  echo "All okay"
fi
exit $ERR
