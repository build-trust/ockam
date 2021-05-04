#!/usr/bin/env bash

source "$(dirname "$0")/common.sh"

change_dir "$OCKAM_RUST"

  # Read and bump versions
  while read line
  do
    read -ra arr <<< "$line"
    CRATE="${arr[0]}"

    # Produce Logs and Diffs
    echo "Gathering changes for $CRATE"
    crate_changes "$CRATE"

    change_dir "$CRATE"
      echo "Bumping $CRATE by ${arr[1]}"
      cargo -q bump "${arr[1]}"
      VERSION="$(crate_version "$CRATE")"
      echo "Updating $CRATE README.md to $VERSION"
      "$SCRIPT_DIR"/upgrade-crate.sh "$PWD/README.md" "$CRATE" "$VERSION"
    pop_dir

    echo "Updating dependants of $CRATE to $VERSION"
    find . -maxdepth 2 -name Cargo.toml -exec "$SCRIPT_DIR/upgrade-crate.sh" '{}' "$CRATE" "$VERSION" \;

    echo "Generating lock files"
    all_crates generate-lockfile
  done < "${1:-/dev/stdin}"

  echo "Checking all crates"
  all_crates check
pop_dir

echo "All updates complete. Testing"
"$PWD/gradlew" test_rust

