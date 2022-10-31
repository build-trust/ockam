#!/usr/bin/env bash

# This script shows release order of our ockam crates. Crates
# are ordered in less-ockam-interdependent order.
#
# There was a bug in cargo-release https://github.com/crate-ci/cargo-release/issues/366
# which gave wrong ordering but has been fixed, so we can use
# cargo-release to release.

val=$(eval "cargo metadata --no-deps | jq '[.packages[] | {name: .name, version: .version, dependencies: .dependencies}]'")
length=$(eval "echo '$val' | jq '. | length' ")
echo "$length"

packages=()
sorted_packages=()

declare -A crates
declare -A sorted_packages_map

for ((c = 0; c < length; c++)); do
  crate_name=$(eval "echo '$val' | jq '.[$c].name' | tr -d '\"' ")

  sorted_packages_map[$crate_name]=false
  IFS=" " read -r -a packages <<<"${packages[*]} $crate_name"
done

for ((c = 0; c < length; c++)); do
  crate_name=$(eval "echo '$val' | jq '.[$c].name' | tr -d '\"' ")
  dependencies=$(eval "echo '$val' | jq '.[$c].dependencies'")
  deps_length=$(eval "echo '$dependencies' | jq '. | length' ")

  declare -A crate"$c"

  for ((d = 0; d < deps_length; d++)); do
    dep=$(eval "echo '$dependencies' | jq '.[$d].name' | tr -d '\"' ")

    set_dep="crate${c}[$dep]=0"

    if [[ -n ${sorted_packages_map[$dep]} ]]; then
      eval "$set_dep"
    fi
  done

  set_crate="crates[$crate_name]=crate$c"
  eval "$set_crate"
done

echo "sorting packages ${packages[*]} ${#packages[@]}"

while [[ -n "${packages[*]}" ]]; do
  index=0

  for package in "${packages[@]}"; do
    deps=$(eval echo \${!"${crates[$package]}"[@]})
    sorted=true

    # Check all package dependencies if there are any
    # that hasn't been indicated to be uploaded.
    for dep in "${deps[@]}"; do
      if [[ ${sorted_packages_map[$dep]} == false ]]; then
        sorted=false
      fi
    done

    if $sorted; then
      echo "-----> $package sorted $index ${#packages[@]}"
      IFS=" " read -r -a sorted_packages <<<"${sorted_packages[*]} $package"
      sorted_packages_map[$package]=true

      # Remove package from packages list
      IFS=" " read -r -a packages <<<"${packages[*]:0:index} ${packages[*]:index+1}"

      echo "Packages left are ${packages[*]}"
      break
    fi
    ((index = index + 1))
  done
done
