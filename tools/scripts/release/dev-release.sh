#!/usr/bin/env bash

source "$(dirname "$0")/common.sh"

change_dir "$OCKAM_RUST"
for CRATE in *
do
  change_dir $CRATE
    cargo -q bump minor
    cargo -q generate-lockfile
    VERSION="$(crate_version "$CRATE")"
    echo "$CRATE $VERSION"
  pop_dir
  find . -maxdepth 2 -name Cargo.toml -exec "$SCRIPT_DIR/upgrade-crate.sh" '{}' "$CRATE" "$VERSION" \;
  all_crates generate-lockfile
done
pop_dir

