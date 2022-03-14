#!/usr/bin/env zsh -e

# This script generates changelog for all Ockam crates that
# are to be published.

source tools/scripts/release/crates-to-publish.sh

for crate in $( echo "$updated_crates" ); do
    # There are crates whose versions are only bumped due to updates
    # in their dependencies, so as not to have empty changelogs we indicate
    # the below message.
    with_commit_msg="feat: updated dependencies"

    # Delete the first 5 lines
    echo "Deleting old changelog in $crate"
    sed -e '1,6d' implementations/rust/ockam/$crate/CHANGELOG.md > CHANGELOG.md.old

    echo "Generating changelog for $crate with tag $last_git_tag"
    git-cliff $last_git_tag.. --with-commit $with_commit_msg --include-path implementations/rust/ockam/$crate/**/*.rs --output implementations/rust/ockam/$crate/CHANGELOG.md

    echo "Updating changelog with recent"
    # Pipe old changelogs
    cat CHANGELOG.md.old >> implementations/rust/ockam/$crate/CHANGELOG.md
    rm CHANGELOG.md.old

    # Replace ## unreleased text to bumped version
    version=$(eval "tomlq package.version -f implementations/rust/ockam/$crate/Cargo.toml")

    search="## unreleased"
    replace="## $version - $(date +'%Y-%m-%d')"
    sed -i -e "s/$search/$replace/" implementations/rust/ockam/$crate/CHANGELOG.md
    rm implementations/rust/ockam/$crate/CHANGELOG.md-e
done

echo "Changelog has been generated. Please review and commit."
