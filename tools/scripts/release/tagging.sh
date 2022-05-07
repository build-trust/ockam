#!/usr/bin/env bash -e

# This script creates tag release on our Ockam repo.

# Commit SHA that release will be based upon.
if [[ -z $COMMIT_SHA ]]; then
    echo "Commit sha variable COMMIT_SHA not set"
    exit 1
fi

# We tag crates using the name and version in Cargo.toml. We
# should ensure we checkout to the specific commit SHA so that
# we use the accurate tag name and version.
#
# Ensure that provided commit SHA is one that we checkout to.
current_commit_sha=$(eval git rev-parse HEAD)

if [[ $current_commit_sha != $COMMIT_SHA ]]; then
    echo "please checkout to specified commit sha"
    exit 1
fi

source tools/scripts/release/crates-to-publish.sh

tag_crate() {
    version=$(eval "tomlq package.version -f implementations/rust/ockam/$1/Cargo.toml")
    name=$(eval "tomlq package.name -f implementations/rust/ockam/$1/Cargo.toml")

    tag="${name}_v${version}"

    echo "Tagging $tag"

    # Check if tag was recently created locally and delete if so.
    # `Draft release` tags are not created upstream till `Release`
    # so we can delete them locally and re-upload on a re-run.
    if git show-ref --tags "$tag" --quiet; then
        echo "Git tag was created recently, deleting now.";
        git tag -d $tag
    fi

    git tag -s $tag $COMMIT_SHA -m "ci: tag $tag"

    text="* [Crate](https://crates.io/crates/$name/$version)
* [Documentation](https://docs.rs/$name/$version/$name/)
* [CHANGELOG](https://github.com/build-trust/ockam/blob/${name}_v$version/implementations/rust/ockam/$name/CHANGELOG.md)";

    gh release create --draft --notes "$text" -t "$name v${version} (rust crate)" "$tag" --target $COMMIT_SHA
}

if [[ ! -z $TAG_SINGLE_CRATE ]]; then
    echo "Tagging only $TAG_SINGLE_CRATE crate"
    tag_crate $TAG_SINGLE_CRATE
    exit 0;
fi

for crate in ${updated_crates[@]}; do
    tag_crate $crate
done
