#!/usr/bin/env bash

# This script generates changelog for all Ockam crates that
# are to be published.

source tools/scripts/release/crates-to-publish.sh

for crate in ${updated_crates[@]}; do
    git cliff --unreleased --commit-path implementations/rust/ockam/$crate --prepend implementations/rust/ockam/$crate/CHANGELOG.md
done
