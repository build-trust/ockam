#!/usr/bin/env bash
set -e

# This script publishes crates to crates.io. Crates that are
# not updated are excluded from cargo release.

if [[ -z $OCKAM_PUBLISH_TOKEN ]]; then
  echo "Publish token variable PUBLISH_TOKEN not set"
  exit 1
fi

# Get the tag before release
GIT_TAG=$(git describe --abbrev=0 --tags $(git rev-list --tags --skip=1 --max-count=1))

source tools/scripts/release/crates-to-publish.sh

declare -A bumped_crates

# Get crates that were updated, this will be published.
for crate in "${updated_crates[@]}"; do
  name=$(eval "tomlq package.name -f $crate/Cargo.toml")
  bumped_crates[$name]=true
done

IFS=" " read -r -a crates_specified_to_be_excluded <<<"$OCKAM_PUBLISH_EXCLUDE_CRATES"
exclude_string=""

# Get crates that are indicated to be excluded.
for crate in "${crates_specified_to_be_excluded[@]}"; do
  echo "Excluding $crate from publishing as specified in env"
  exclude_string="$exclude_string --exclude $crate"
  bumped_crates[$crate]=false
done

declare -A crates_version

# Every other crate that were not updated...
for crate in implementations/rust/ockam/*; do
  if [[ -f $crate ]]; then
    echo "$crate is a file, skipping."
    continue
  fi

  # Check if there is a Cargo.toml file in dir
  if [[ ! -f "$crate/Cargo.toml" ]]; then
    echo "echo "$crate is not a crate.""
    continue
  fi

  # There are some crates that differ from their folder name, e.g. ockam_ffi
  # so we need the crate name source of truth from Cargo.toml.
  name=$(eval "tomlq package.name -f $crate/Cargo.toml")
  version=$(eval "tomlq package.version -f $crate/Cargo.toml")
  crates_version[$name]="$version"

  # Add crates that have not been updated and not recently excluded to excluded crate.
  if [[ -z ${bumped_crates[$name]} ]]; then
    echo "Excluding $name from publishing"
    exclude_string="$exclude_string --exclude $name"
    bumped_crates[$name]=false
  fi
done

# Check if this is a re-run...
if [[ $OCKAM_PUBLISH_RECENT_FAILURE == true ]]; then
  echo "Script rerun on recent failure..."
  echo "Checking recently successfully published crates..."

  for i in "${!bumped_crates[@]}"; do
    if [[ ${bumped_crates[$i]} == true ]]; then
      echo "Checking if $i version ${crates_version[$i]} has been published recently...."
      val=$(cargo search $i --limit 1)

      if [[ $val == *"$i = \"${crates_version[$i]}\""* ]]; then
        exclude_string="$exclude_string --exclude $i"
        echo "$i already published. Excluded crate from being published."
      fi
    fi
  done
fi

cargo release release --no-confirm --config tools/scripts/release/release.toml --no-tag --no-verify $exclude_string --token "$OCKAM_PUBLISH_TOKEN" --execute
