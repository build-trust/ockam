#!/usr/bin/env bash -e

# This script checks for crates that have been modified
# compared to last created tag. It is to be used with other
# scripts to generate changelog, bump and publish.

# Check if file has been updated since last tag.
last_git_tag=$(eval "git describe --tags --abbrev=0");
updated_crates="";

if [[ ! -z $GIT_TAG ]]; then
    # Check if git tag is valid.
    if git show-ref --tags $GIT_TAG --quiet; then
        echo "Specified $GIT_TAG as tag to track updated crates.";
        last_git_tag=$GIT_TAG
    else
        echo "Specified git tag used to track updated crates invalid."
        exit 1
    fi
fi

for crate in $(ls "implementations/rust/ockam"); do
    if [[ -f implementations/rust/ockam/$crate ]]; then
        echo "$crate is a file, skipping."
        continue
    fi

    is_publish=$(tomlq package.publish -f implementations/rust/ockam/$crate/Cargo.toml)
    if [[ is_publish == false ]]; then
        echo "$crate indicate as not-publish"
        continue
    fi

    if git diff $last_git_tag --quiet --name-status -- implementations/rust/ockam/$crate/src; then
        git diff $last_git_tag --quiet --name-status -- implementations/rust/ockam/$crate/Cargo.toml || updated_crates="$updated_crates $crate"
    else
        updated_crates="$updated_crates $crate"
    fi
done
