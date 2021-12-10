#!/usr/bin/env bash

# This script checks for crates that have been modified
# compared to last created tag. It is to be used with other
# scripts to generate changelog, bump and publish.

# Check if file has been updated since last tag.
last_git_tag=$(eval "git describe --tags --abbrev=0");
updated_crates="";

for crate in $(ls "implementations/rust/ockam"); do
    if git diff $last_git_tag --quiet --name-status -- implementations/rust/ockam/$crate/src; then
        git diff $last_git_tag --quiet --name-status -- implementations/rust/ockam/$crate/Cargo.toml || updated_crates="$updated_crates $crate"
    else
        updated_crates="$updated_crates $crate"
    fi
done
