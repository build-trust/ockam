#!/usr/bin/env bash

source "$(dirname "$0")"/common.sh

function crate_changes {
  CRATE="$1"
  SHA="$2"
  CRATE_DIR="$OCKAM_RUST$CRATE"
  change_dir "$OCKAM_RUST"
  SPAN="$SHA..HEAD"
  CHANGES="$CRATE_DIR/Changelog-INCOMING.md"
  VERSION=$(crate_version $CRATE)
  printf "## v%s - $(date -I)\n### Changed\n- Dependencies updated\n\n"  "$VERSION" >"$CHANGES"
  git log --oneline --pretty=reference "$SPAN" "$CRATE_DIR" | perl -pe 's/^[\w\d]+\s+\(|,\s+.*?\)$//g' >>"$CHANGES"
  cat "$CHANGES"
}

crate_changes "$1" "$2"
