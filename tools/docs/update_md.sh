set -e
if [ -z "$OCKAM_HOME" ]; then
  echo "Please set OCKAM_HOME to the repo root"
  exit 1
fi

if [ -z "$DOCS_HOME" ]; then
  echo "Please set DOCS_HOME to the repo root"
  exit 1
fi

# Install example_blocks binary
echo "Building example_blocks utility"
pushd "$OCKAM_HOME/tools/docs/example_blocks" &>/dev/null || exit
cargo -q install --path .
popd &>/dev/null || exit

# Look for md files from this directory
for FILE_NAME in $(find "$DOCS_HOME" -type f -name "*.md"); do
    echo "==> $FILE_NAME"
    TMP=$(mktemp)
    EXAMPLES_DIR="$OCKAM_HOME/examples/rust/get_started" example_blocks "$FILE_NAME" >"$TMP"
    cat "$TMP" >"$FILE_NAME"
done
