#!/usr/bin/env bash
set -e

# This script generates changelog for all Ockam crates that
# are to be published.

function generate_changelog() {
  crate="$1"
  from_released_git_tag="$2"

  # There are crates whose versions are only bumped due to updates
  # in their dependencies, so as not to have empty changelogs we indicate
  # the below message.
  echo "Generating changelog for $crate with tag $last_git_tag"
  with_commit_msg="feat: updated dependencies"
  git-cliff "$from_released_git_tag.." --config tools/cliff/cliff.toml --with-commit "$with_commit_msg" --include-path "$crate/**/*.rs" --prepend "$crate/CHANGELOG.md"
  # Replace ## unreleased text to bumped version
  version=$(eval "tomlq package.version -f $crate/Cargo.toml")

  search="## unreleased"
  replace="## $version - $(date +'%Y-%m-%d')"
  sed -i -e "s/$search/$replace/" "$crate"/CHANGELOG.md
}

if [[ -z $GIT_TAG_WE_WILL_BE_UPDATING ]]; then
  source tools/scripts/release/crates-to-publish.sh
  for crate in "${updated_crates[@]}"; do
    generate_changelog "$crate" "$last_git_tag"
  done

  echo "Changelog has been generated. Please review and commit."
fi
