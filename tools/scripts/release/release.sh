#!/usr/bin/env bash

source "$(dirname "$0")/common.sh"

change_dir "$OCKAM_RUST"

  # Read and bump versions
  while read line
  do
    read -ra arr <<< "$line"
    CRATE="${arr[0]}"

    change_dir "$CRATE"
      export VERSION=$(crate_version $CRATE)
      echo "Updating $CRATE README.md to $VERSION"
      "$SCRIPT_DIR"/upgrade-crate.sh "$PWD/README.md" "$CRATE" "$VERSION"
    pop_dir

    echo "Updating dependants of $CRATE to $VERSION"
    find . -maxdepth 2 -name Cargo.toml -exec "$SCRIPT_DIR/upgrade-crate.sh" '{}' "$CRATE" "$VERSION" \;
  done < "${1:-/dev/stdin}"

  echo "Generating lock files for crates"
  all_crates generate-lockfile

  echo "Generate lock files for examples"
  change_dir "$OCKAM_HOME/examples/rust/get_started"
  cargo -q generate-lockfile
  pop_dir
  echo "Checking all crates"
  all_crates check
pop_dir

echo "All updates complete. Testing"
"$PWD/gradlew" test_rust

