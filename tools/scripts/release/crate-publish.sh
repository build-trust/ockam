#!/usr/bin/env bash

# This script publishes crates to crates.io. Crates that are
# not updated are excluded from cargo release.

if [[ -z $PUBLISH_TOKEN ]]; then
    echo "Publish token variable PUBLISH_TOKEN not set"
fi

source tools/scripts/release/crates-to-publish.sh

declare -A bumped_crates

for crate in ${updated_crates[@]}; do
    bumped_crates[$crate]=true
done

exclude_string=""

for crate in $(ls "implementations/rust/ockam"); do
    # Add crate to excluded crate
    if [[ -z ${bumped_crates[$crate]} ]]; then
        echo "Excluding $crate from publishing"
        exclude_string="$exclude_string --exclude $crate";
    fi
done

echo y | cargo release release --no-tag --no-verify --no-dev-version $exclude_string --token $PUBLISH_TOKEN --execute;
