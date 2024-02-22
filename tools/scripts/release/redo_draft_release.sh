#!/usr/bin/env bash
set -e

if [[ -z $LAST_RELEASED_TAG ]]; then
  echo "Last production release tag must be indicated"
  exit 1
fi

if [[ -z $GIT_TAG_WE_WILL_BE_UPDATING ]]; then
  echo "Last production release tag must be indicated"
  exit 1
fi

function get_crates_to_update() {
  unset updated_crates_from_non_released_tag
  unset updated_crates_from_released_tag

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

    if git diff "$LAST_RELEASED_TAG..$GIT_TAG_WE_WILL_BE_UPDATING" --quiet --name-status -- "$crate"/src; then
      git diff "$LAST_RELEASED_TAG..$GIT_TAG_WE_WILL_BE_UPDATING" --quiet --name-status -- "$crate"/Cargo.toml || updated_crates_from_released_tag="$updated_crates_from_released_tag $crate "
    else
      updated_crates_from_released_tag="$updated_crates_from_released_tag $crate "
    fi

    if git diff "$GIT_TAG_WE_WILL_BE_UPDATING" --quiet --name-status -- "$crate"/src; then
      git diff "$GIT_TAG_WE_WILL_BE_UPDATING" --quiet --name-status -- "$crate"/Cargo.toml || updated_crates_from_non_released_tag="$updated_crates_from_non_released_tag $crate "
    else
      updated_crates_from_non_released_tag="$updated_crates_from_non_released_tag $crate "
    fi
  done
}


# - For cargo.toml bump, perform a no-version bump if a crate is updated in from the last published and unpublished release
# - For changelog, delete the old changelog and generate a new one
get_crates_to_update
initial_updated_crates_from_non_released_tag=""

while [[ "$initial_updated_crates_from_non_released_tag" != "$updated_crates_from_non_released_tag" ]]; do
  initial_updated_crates_from_non_released_tag="$updated_crates_from_non_released_tag"
  IFS=" " read -r -a updated_crates_from_non_released_tag <<<"${updated_crates_from_non_released_tag[*]}"

  for crate in "${updated_crates_from_non_released_tag[@]}"; do
    if [[ "$updated_crates_from_released_tag" == *"$crate"* ]]; then
      # Perform a release version only bump crates that uses $crate
      echo y | cargo release release --config tools/scripts/release/release.toml --no-push --no-publish --no-tag --no-dev-version --package "$crate" --execute
    elif
      # Perform a minor release
    fi

    ## Delete old changelog and create a new changelog


  done
  get_crates_to_update
done