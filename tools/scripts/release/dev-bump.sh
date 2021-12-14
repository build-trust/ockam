#!/usr/bin/env bash

for crate in $(ls "implementations/rust/ockam"); do
    # Check if crate is a -dev version
    version=$(eval "tomlq package.version -f implementations/rust/ockam/$crate/Cargo.toml")

    if [[ "$version" != *"-dev"* ]]; then
        echo "bumping crate $crate to dev"
        echo y | cargo release release --no-push --no-publish --no-tag --package $crate --execute
    fi
done
