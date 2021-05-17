#!/usr/bin/env bash

source "$(dirname "$0")"/common.sh

function crate_changes {
  CRATE="$1"
  OLD_VERSION="$2"
  NEW_VERSION="$3"
  CRATE_DIR="$OCKAM_RUST$CRATE"
  change_dir "$OCKAM_RUST"
  SPAN="${CRATE}_v$OLD_VERSION..HEAD"
  CHANGES="$CRATE_DIR/Changelog-INCOMING.md"
  printf "## v%s - $(date -I)\n### Added\n### Changed\n### Deleted\n\n"  "$NEW_VERSION" >"$CHANGES"
  git log --oneline --pretty=reference "$SPAN" "$CRATE_DIR" | perl -pe 's/^[\w\d]+\s+\(|,\s+.*?\)$//g' >>"$CHANGES"
  cat "$CHANGES"
}

crate_changes "$1" "$2" "$3"
