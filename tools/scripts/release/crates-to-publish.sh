#!/usr/bin/env bash

# Check if file has been updated since last tag.
last_git_tag=$(eval "git describe --tags --abbrev=0");
updated_crates="";

for path in $(ls "implementations/rust/ockam"); do
    if git diff $last_git_tag --quiet --name-status -- implementations/rust/ockam/$path/src; then
        git diff $last_git_tag --quiet --name-status -- implementations/rust/ockam/$path/Cargo.toml || updated_crates="$updated_crates $path"
    else
        updated_crates="$updated_crates $path"
    fi
done
