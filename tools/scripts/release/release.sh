#!/usr/bin/env bash
set -e

GITHUB_USERNAME=$(gh api user | jq -r '.login')
OWNER="metaclips"
RELEASE_NAME="release_$(date +'%d-%m-%Y')_$(date +'%s')"

source tools/scripts/release/helpers/flags.sh
source tools/scripts/release/helpers/check_gh_scopes.sh
source tools/scripts/release/helpers/log_pipe.sh
source tools/scripts/release/helpers/check_executables.sh

source tools/scripts/release/helpers/approve_ci.sh
source tools/scripts/release/helpers/crate_bump.sh
source tools/scripts/release/helpers/crate_release.sh
source tools/scripts/release/helpers/binaries_release.sh
source tools/scripts/release/helpers/package_release.sh
source tools/scripts/release/helpers/homebrew_bump.sh
source tools/scripts/release/helpers/terraform_bump.sh
source tools/scripts/release/helpers/terraform_binaries_release.sh

set -x

function dialog_info() {
  echo -e "\033[01;33m$1\033[00m"
}

function success_info() {
  echo -e "\033[01;32m$1\033[00m"
}

if [[ -z $IS_DRAFT_RELEASE ]]; then
  dialog_info "Please indicate flag if script is to run in draft or release mode.
Run ./release.sh -h to find all available flags."
  exit 1
fi

#------------------------------------------------------------------------------------------------------------------------------------------------------------------#
# Perform Ockam bump and binary draft release if specified
if [[ $IS_DRAFT_RELEASE == true ]]; then
  # Bump Ockam version and create PR
  echo "Starting Ockam crate bump"
  ockam_bump
  success_info "Crate bump pull request created.... Starting Ockam crates.io publish."

  # Create Ockam binaries as draft
  echo "Releasing Ockam draft binaries"
  release_ockam_binaries
  success_info "Draft release has been created.... Starting Homebrew release."
fi

# Get latest tag
if [[ -z $LATEST_TAG_NAME ]]; then
  latest_tag_name=$(gh api -H "Accept: application/vnd.github+json" /repos/$OWNER/ockam/releases | jq -r .[0].tag_name)
  if [[ $latest_tag_name != *"ockam_v"* ]]; then
    echo "Invalid Git Tag retrieved"
    exit 1
  fi

  success_info "Latest tag is $latest_tag_name"
else
  latest_tag_name="$LATEST_TAG_NAME"
fi

# Release draft to production
if [[ $IS_DRAFT_RELEASE == true ]]; then
  # Get File hash from draft release
  echo "Retrieving Ockam file SHA"
  file_and_sha=""

  temp_dir=$(mktemp -d)
  pushd $temp_dir
  gh release download $latest_tag_name -R $OWNER/ockam

  # TODO Ensure that SHA are cosign verified
  while read -r line; do
    file=($line)
    if [[ ${file[1]} == *".so"* || ${file[1]} == *".sig"* ]]; then
      continue
    fi

    file_and_sha="$file_and_sha ${file[1]}:${file[0]}"
  done < sha256sums.txt
  popd
  rm -rf /tmp/$RELEASE_NAME

  echo "File and hash are $file_and_sha"

  echo "Releasing Ockam container image"
  release_ockam_package $latest_tag_name "$file_and_sha" false
  success_info "Ockam package draft release successful.... Starting Homebrew release"

  # Homebrew version bump
  echo "Bumping Homebrew"
  homebrew_repo_bump $latest_tag_name "$file_and_sha"
  success_info "Homebrew release successful.... Starting Terraform Release"

  # Terraform version bump
  echo "Bumping Terraform"
  terraform_repo_bump $latest_tag_name

  # Terraform binary release
  echo "Releasing Ockam Terraform binaries"
  terraform_binaries_release $latest_tag_name

  success_info "Terraform release successful"
fi

#---------------------------------------Start production release------------------------------------------------#
if [[ $IS_DRAFT_RELEASE == false ]]; then
  # Make Ockam Github draft as latest.
  echo "Releasing Ockam Github release"
  gh release edit $latest_tag_name --draft=false -R $OWNER/ockam

  # Release Terraform Github release
  terraform_tag=${latest_tag_name:6}
  gh release edit $terraform_tag --draft=false -R $OWNER/terraform-provider-ockam

  # Release Ockam package
  echo "Making Ockam container latest"
  release_ockam_package $latest_tag_name "nil" true
  delete_if_draft_package_exists $latest_tag_name
  success_info "Ockam package release successful."

  # Release Ockam crates to crates.io
  OCKAM_PUBLISH_RECENT_FAILURE=$RECENT_FAILURE

  echo "Starting Crates IO publish"
  ockam_crate_release
  success_info "Crates.io publish successful."

  success_info "Release Done 🚀🚀🚀."
fi

exit 0
