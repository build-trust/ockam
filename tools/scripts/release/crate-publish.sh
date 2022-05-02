#!/usr/bin/env bash -e

# This script publishes crates to crates.io. Crates that are
# not updated are excluded from cargo release.

if [[ -z $PUBLISH_TOKEN ]]; then
    echo "Publish token variable PUBLISH_TOKEN not set"
    exit 1
fi

source tools/scripts/release/crates-to-publish.sh

declare -A bumped_crates

for crate in ${updated_crates[@]}; do
    name=$(eval "tomlq package.name -f implementations/rust/ockam/$crate/Cargo.toml")
    bumped_crates[$name]=true
done

crates_specified_to_be_excluded=( $EXCLUDE_CRATES )
exclude_string=""

for crate in ${crates_specified_to_be_excluded[@]}; do
    echo "Excluding $crate from publishing as specified in env"
    exclude_string="$exclude_string --exclude $crate"
    bumped_crates[$crate]=false
done

declare -A crates_version

for crate in $(ls "implementations/rust/ockam"); do
    # There are some crates that differ from their folder name, e.g. ockam_ffi
    # so we need the crate name source of truth from Cargo.toml.
    name=$(eval "tomlq package.name -f implementations/rust/ockam/$crate/Cargo.toml")
    version=$(eval "tomlq package.version -f implementations/rust/ockam/$crate/Cargo.toml")
    crates_version[$name]="$version"

    # Add crates that have not been updated and not recently excluded to excluded crate.
    if [[ -z ${bumped_crates[$name]} ]]; then
        echo "Excluding $name from publishing"
        exclude_string="$exclude_string --exclude $name";
        bumped_crates[$name]=false
    fi
done

if [[ ! -z $RECENT_FAILURE ]]; then
    echo "Script rerun on recent failure..."
    echo "Checking recently successfully published crates..."

    for i in "${!bumped_crates[@]}"; do
        if [[ ${bumped_crates[$i]} == true ]]; then
            echo "Checking if $i version ${crates_version[$i]} has been published recently...."
            val=$(cargo search $i --limit 1)

            if [[ $val == *"$i = \"${crates_version[$i]}\""* ]]; then
                exclude_string="$exclude_string --exclude $i";
                echo "$i already published. Excluded crate from being published."
            fi
        fi
    done
fi

echo y | cargo release release --no-tag --no-verify --no-dev-version $exclude_string --token $PUBLISH_TOKEN --execute;
