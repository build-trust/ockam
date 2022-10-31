#!/usr/bin/env bash
set -e

# This script generates changelog for all Ockam crates that
# are to be published.

source tools/scripts/release/crates-to-publish.sh

for crate in $(echo "$updated_crates"); do
  # There are crates whose versions are only bumped due to updates
  # in their dependencies, so as not to have empty changelogs we indicate
  # the below message.
  echo "Generating changelog for $crate with tag $last_git_tag"
  with_commit_msg="feat: updated dependencies"
  git-cliff "$last_git_tag".. --config tools/cliff/cliff.toml --with-commit "$with_commit_msg" --include-path "$crate"/**/*.rs --prepend "$crate"/CHANGELOG.md
  # Replace ## unreleased text to bumped version
  version=$(eval "tomlq package.version -f $crate/Cargo.toml")

  search="## unreleased"
  replace="## $version - $(date +'%Y-%m-%d')"
  sed -i -e "s/$search/$replace/" "$crate"/CHANGELOG.md
done

echo "Changelog has been generated. Please review and commit."
