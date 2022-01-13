#!/usr/bin/env bash

# This script publishes crates to crates.io. Crates that are
# not updated are excluded from cargo release.

if [[ -z $PUBLISH_TOKEN ]]; then
    echo "Publish token variable PUBLISH_TOKEN not set"
    exit 1
fi

source tools/scripts/release/crates-to-publish.sh

declare -A bumped_crates

for crate in ${updated_crates[@]}; do
    bumped_crates[$crate]=true
done

crates_specified_to_be_excluded=( $EXCLUDE_CRATES )
exclude_string=""

for crate in ${crates_specified_to_be_excluded[@]}; do
    echo "Excluding $crate from publishing as specified in env"
    exclude_string="$exclude_string --exclude $crate"
    bumped_crates[$crate]=false
done

for crate in $(ls "implementations/rust/ockam"); do
    # Add crate to excluded crate
    if [[ -z ${bumped_crates[$crate]} ]]; then
        name=$(eval "tomlq package.name -f implementations/rust/ockam/$crate/Cargo.toml")
        echo "Excluding $name from publishing"
        exclude_string="$exclude_string --exclude $name";
    fi
done

echo y | cargo release release --no-tag --no-verify --no-dev-version $exclude_string --token $PUBLISH_TOKEN --execute;
