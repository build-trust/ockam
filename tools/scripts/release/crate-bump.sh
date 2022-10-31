#!/usr/bin/env bash
set -e

# This script bumps all crates that have been updated compared to
# last git tag. OCKAM_BUMP_RELEASE_VERSION value is to be set to indicate the
# release version of all crates (usually minor). If there are crates
# that are not to follow the OCKAM_BUMP_RELEASE_VERSION value, we can further
# set MODIFIED_RELEASE value to indicates individual crates and how
# they are to be bumped "signature_core:minor ockam:major" signature_core
# crate will be bumped as a minor and ockam crate will be bumped as
# major.

if [[ -z $OCKAM_BUMP_RELEASE_VERSION ]]; then
  echo "please set OCKAM_BUMP_RELEASE_VERSION variable"
  exit 1
fi

if [[ -z $OCKAM_BUMP_BUMPED_DEP_CRATES_VERSION ]]; then
  echo "Version of bumped transitive dependencies set to minor"
  OCKAM_BUMP_BUMPED_DEP_CRATES_VERSION="minor"
fi

declare -A specified_crate_version
IFS=" " read -r -a crate_array <<<"$OCKAM_BUMP_MODIFIED_RELEASE"

for word in "${crate_array[@]}"; do
  key="${word%%:*}"
  value="${word##*:}"
  specified_crate_version[$key]=$value
done

declare -A bumped_crates

recently_updated_crates=""

source tools/scripts/release/crates-to-publish.sh
bumping_transitive_deps=false

# With special case like below
# crateA -> crateB -> crateC -> crateD
# Where -> means "is a dependency of", we need to still bump crates
# whose transitive dependency is updated.
# We keep a state of recently updated crates and then recursively match
# it with the new state of updated crate and only exit if all crates have
# been bumped.
#
# If crate A version is bumped and its updated version is changed in the
# Cargo.toml of crateB, this script then re-runs, checking if there are any
# recently updated crate also keeping the data of recently updated crates (crateA)
# we then compare the new state of updated crates with that of the old one and
# in this scenario getting crateB whose deps were recently modified. Seeing there's an
# updated crate (crateB) we then bump all crates ignoring recently bumped
# crates (crateA so as not to bump twice) then we recursively check again if there
# are any newly updated/modified crates whose version has not been bumped till new
# state is same as old state.
#
# Case 2
# crateF -> crateC
# crateA -> crateB -> crateC
# The script bumps (crateF and crateA) version and (crateC and crateB) `deps version`,
# on the second iteration, (crateC and crateB) version is then bumped, on the third iteration
# we do not bump crateC version even though its dep has been modified as it's version has already
# been bumped for a release.
while [[ "$updated_crates" != "$recently_updated_crates" ]]; do
  for crate in "${updated_crates[@]}"; do
    if [[ -n "${bumped_crates[$crate]}" ]]; then
      echo "===> $crate has been bumped recently ignoring"
      continue
    fi

    version=$OCKAM_BUMP_RELEASE_VERSION
    if [[ $bumping_transitive_deps == true ]]; then
      version=$OCKAM_BUMP_BUMPED_DEP_CRATES_VERSION
      echo "Bumping transitive dependent crate $crate version to $version"
    fi

    name=$(eval "tomlq package.name -f $crate/Cargo.toml")

    # Check if crate version was specified manually
    if [[ -n "${specified_crate_version[$crate]}" ]]; then
      echo "Bumping $crate version specified manually as ${specified_crate_version[$crate]}"
      version="${specified_crate_version[$crate]}"
    fi

    bumped_crates[$crate]=true

    echo "Bumping $crate crate"
    echo y | cargo release "$version" --config tools/scripts/release/release.toml --no-push --no-publish --no-tag --no-dev-version --package "$name" --execute
  done

  recently_updated_crates=$updated_crates
  bumping_transitive_deps=true

  source tools/scripts/release/crates-to-publish.sh
  printf "Recently bumped crates are %s \n updated crates are %s" "$recently_updated_crates" "$updated_crates"
done

echo "Bumped crates $recently_updated_crates"
