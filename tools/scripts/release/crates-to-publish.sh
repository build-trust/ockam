#!/usr/bin/env bash
set -e

# This script checks for crates that have been modified
# compared to last created tag. It is to be used with other
# scripts to generate changelog, bump and publish.

# Check if file has been updated since last tag.
last_git_tag=$(eval "git describe --tags --abbrev=0")
updated_crates=""

if [[ -n $GIT_TAG ]]; then
  # Check if git tag is valid.
  if git show-ref --tags "$GIT_TAG" --quiet; then
    echo "Specified $GIT_TAG as tag to track updated crates."
    last_git_tag=$GIT_TAG
  else
    echo "Specified git tag used to track updated crates invalid."
    exit 1
  fi
fi

for crate in implementations/rust/ockam/*; do
  if [[ -f $crate ]]; then
    echo "$crate is a file, skipping."
    continue
  fi

  # Ensure that folder contains rust crate.
  if [[ ! -f "$crate/Cargo.toml" ]]; then
    echo "$crate is not a crate, skipping"
    continue
  fi

  is_publish=$(tomlq package.publish -f "$crate"/Cargo.toml)
  if [[ $is_publish == false ]]; then
    echo "$crate indicate as not-publish"
    continue
  fi

  # Check if the src file is updated, if it isn't check if the Cargo.toml file is updated.
  if git diff "$last_git_tag" --quiet --name-status -- "$crate"/src; then
    git diff "$last_git_tag" --quiet --name-status -- "$crate"/Cargo.toml || updated_crates="$updated_crates $crate "
  else
    updated_crates="$updated_crates $crate "
  fi
done

crates_that_must_be_bumped=("ockam" "ockam_app_lib" "ockam_command")
for crate in "${crates_that_must_be_bumped[@]}"; do
  if [[ $updated_crates == *"implementations/rust/ockam/$crate "* ]]; then
    continue
  fi

  echo "$crate wasn't updated but is intended to be released"
  updated_crates="$updated_crates implementations/rust/ockam/$crate"
done

IFS=" " read -r -a updated_crates <<<"${updated_crates[*]}"
