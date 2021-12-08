#!/opt/homebrew/bin/bash

# Publishes crates also taking note of Ockam inter-dependency
# order.
#
# Packages are ordered in order of least dependency using publish-order.sh
# script.

source tools/scripts/release/crates-to-publish.sh

declare -A crates_to_publish

echo "${updated_crates[@]}"
for crate in ${updated_crates[@]}; do
    echo "Not Publishing $crate"
    crates_to_publish[$crate]=true
done

source tools/scripts/release/publish-order.sh

for package in ${sorted_packages[@]}; do
    # Check if the package is in the list of
    # crates to be published.
    if [[ -z ${crates_to_publish[$package]} ]]; then
        echo "Not Publishing $package"
        continue
    fi

    echo "Publishing $package"
done
