#!/usr/bin/env bash

source tools/scripts/release/crates-to-publish.sh

# Commit SHA that release will be based upon.
if [[ -z $COMMIT_SHA ]]; then
    echo "Commit sha variable COMMIT_SHA not set"
fi

for crate in ${updated_crates[@]}; do
    version=$(eval "tomlq package.version -f implementations/rust/ockam/$crate/Cargo.toml")
    tag="${crate}_v${version}"

    echo "Tagging $tag"
    git tag -s $tag -m "ci: tag $tag"

    text="* [Crate](https://crates.io/crates/$crate/$version)
    * [Documentation](https://docs.rs/$crate/$version/$crate/)
    * [CHANGELOG](https://github.com/ockam-network/ockam/blob/${crate}_$version/implementations/rust/ockam/$crate/CHANGELOG.md)";

    gh release create --draft --notes "$text" -t "$crate $version (rust crate)" "$tag" --target $COMMIT_SHA
done
