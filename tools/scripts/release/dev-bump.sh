#!/usr/bin/env bash

for crate in $(ls "implementations/rust/ockam"); do
    # Check if crate is a -dev version
    version=$(eval "tomlq package.version -f implementations/rust/ockam/$crate/Cargo.toml")
    name=$(eval "tomlq package.name -f implementations/rust/ockam/$crate/Cargo.toml")

    if [[ "$version" != *"-dev"* ]]; then
        echo "bumping crate $name to dev"
        echo y | cargo release release --no-push --no-publish --no-tag --package $name --execute
    fi
done
