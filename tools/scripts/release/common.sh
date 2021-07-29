#!/usr/bin/env bash

if [ -z "$OCKAM_HOME" ]
then
  echo "Please set the OCKAM_HOME environment variable to the ockam repository root directory."
  exit 0
fi

OCKAM_RUST="$OCKAM_HOME/implementations/rust/ockam/"
SCRIPT_DIR="$OCKAM_HOME/tools/scripts/release"
export OCKAM_RUST
export SCRIPT_DIR

function change_dir {
  pushd "$1" >/dev/null || exit 1
}

function pop_dir {
  popd >/dev/null || exit 1
}

function crate_version {
  perl -ne '/^version = "([^"]+)"/ and print "$1\n"' < "$OCKAM_RUST/$1/Cargo.toml"
}

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
  pop_dir
}

function all_crates {
    change_dir "$OCKAM_RUST"
    for CRATE in *
    do
      if [[ ! -d $CRATE ]]; then
          continue;
      fi
      change_dir "$CRATE"
      echo "all_crates: $CRATE $*"
      cargo -q $* 1>/dev/null
      pop_dir
    done
    pop_dir
}

