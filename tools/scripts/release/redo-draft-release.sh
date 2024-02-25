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

if [[ -z $OCKAM_BUMP_BUMPED_DEP_CRATES_VERSION ]]; then
  echo "Version of bumped transitive dependencies set to minor"
  OCKAM_BUMP_BUMPED_DEP_CRATES_VERSION="minor"
fi

function get_crates_to_update() {
  unset crates_updated_after_last_draft
  unset crates_updated_on_last_draft

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
      git diff "$LAST_RELEASED_TAG..$GIT_TAG_WE_WILL_BE_UPDATING" --quiet --name-status -- "$crate"/Cargo.toml || crates_updated_on_last_draft="$crates_updated_on_last_draft $crate "
    else
      crates_updated_on_last_draft="$crates_updated_on_last_draft $crate "
    fi

    if git diff "$GIT_TAG_WE_WILL_BE_UPDATING" --quiet --name-status -- "$crate"/src; then
      git diff "$GIT_TAG_WE_WILL_BE_UPDATING" --quiet --name-status -- "$crate"/Cargo.toml || crates_updated_after_last_draft="$crates_updated_after_last_draft $crate "
    else
      crates_updated_after_last_draft="$crates_updated_after_last_draft $crate "
    fi
  done
}

echo "Bumping updated crates"
source tools/scripts/release/changelog.sh
get_crates_to_update
initial_crates_updated_after_last_draft=""
declare -A bumped_crates

while [[ "$initial_crates_updated_after_last_draft" != "$crates_updated_after_last_draft" ]]; do
  initial_crates_updated_after_last_draft="$crates_updated_after_last_draft"
  IFS=" " read -r -a crates_updated_after_last_draft <<<"${crates_updated_after_last_draft[*]}"

  for crate in "${crates_updated_after_last_draft[@]}"; do
    crate_name=$(eval "tomlq package.name -f $crate/Cargo.toml")

    if [[ -n "${bumped_crates[$crate_name]}" ]]; then
      echo "===> $crate_name has been bumped recently, ignoring"
      continue
    fi

    bumped_crates[$crate_name]=true

    if [[ "$crates_updated_on_last_draft" == *"$crate"* ]]; then
      # Perform a release version which only bump crates that uses $crate
      echo y | cargo release release --config tools/scripts/release/release.toml --no-push --no-publish --no-tag --no-dev-version --package "$crate_name" --execute
    else
      # Perform a minor release
      echo y | cargo release "$OCKAM_BUMP_BUMPED_DEP_CRATES_VERSION" --config tools/scripts/release/release.toml --no-push --no-publish --no-tag --no-dev-version --package "$crate_name" --execute
    fi

    ## Delete old changelog if it was created during last draft release
    if [[ "$crates_updated_on_last_draft" == *"$crate"* ]]; then
      CHANGELOG_FILE_PATH="$crate/CHANGELOG.md" python3 tools/scripts/release/delete_last_changelog.py
    fi

    generate_changelog "$crate" "$LAST_RELEASED_TAG"
    git add --all
    git commit -m "ci: update changelog for $crate_name"
  done

  get_crates_to_update
done
