#!/usr/bin/env bash

source tools/scripts/release/crates-to-publish.sh

declare -A specified_crate_version

crate_array=($MODIFIED_RELEASE)

for word in ${crate_array[@]}; do
    key="${word%%:*}"
    value="${word##*:}"
    specified_crate_version[$key]=$value
done

for to_update in ${updated_crates[@]}; do
    if [[ $DEV_VERSION == true ]]; then
        cargo release release --no-push --no-publish --no-confirm --no-tag --package $to_update --execute
    else

        # If the bump version is indicated as release, we don't bump
        # or publish the crate.
        version=$RELEASE_VERSION
        if [[ ! -z "${specified_crate_version[$to_update]}" ]]; then
            echo "bumping $to_update as ${specified_crate_version[$to_update]}"
            version="${specified_crate_version[$to_update]}"
        fi

        cargo release $version --no-push --no-publish --no-confirm --no-tag --no-dev-version --package $to_update --execute
    fi
done
