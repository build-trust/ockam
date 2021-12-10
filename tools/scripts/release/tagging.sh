#!/usr/bin/env bash

# This script creates tag release on our Ockam repo.

# Commit SHA that release will be based upon.
if [[ -z $COMMIT_SHA ]]; then
    echo "Commit sha variable COMMIT_SHA not set"
    exit 1
fi

source tools/scripts/release/crates-to-publish.sh

for crate in ${updated_crates[@]}; do
    version=$(eval "tomlq package.version -f implementations/rust/ockam/$crate/Cargo.toml")
    name=$(eval "tomlq package.name -f implementations/rust/ockam/$crate/Cargo.toml")

    tag="${name}_v${version}"

    echo "Tagging $tag"

    git tag -s $tag $COMMIT_SHA -m "ci: tag $tag"

    text="* [Crate](https://crates.io/crates/$name/$version)
* [Documentation](https://docs.rs/$name/$version/$name/)
* [CHANGELOG](https://github.com/ockam-network/ockam/blob/${name}_v$version/implementations/rust/ockam/$name/CHANGELOG.md)";

    gh release create --draft --notes "$text" -t "$name v${version} (rust crate)" "$tag" --target $COMMIT_SHA
done
