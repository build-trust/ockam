#!/usr/bin/env bash
set -ex

if [[ -z $GITHUB_USERNAME ]]; then
  echo "Please set your github username"
  exit 1
fi

owner="build-trust"

function ockam_bump() {
  gh workflow run create-release-pull-request.yml --ref develop\
    -F git_tag="$GIT_TAG" -F modified_release="$MODIFIED_RELEASE"\
    -F release_version="$RELEASE_VERSION" -F bumped_dep_crates_version="$BUMPED_DEP_CRATES_VERSION"\
    -R $owner/ockam

  # Sleep for 10 seconds to ensure we are not affected by Github API downtime.
  sleep 10
  # Wait for workflow run
  run_id=$(gh run list --workflow=create-release-pull-request.yml -b develop -u $GITHUB_USERNAME -L 1 -R $owner/ockam --json databaseId | jq -r .[0].databaseId)
  gh run watch $run_id --exit-status -R $owner/ockam
}

function ockam_crate_release() {
  gh workflow run publish_crates.yml --ref develop \
    -F git_tag="$GIT_TAG" -F exclude_crates="$EXCLUDE_CRATES" \
    -F recent_failure="$RECENT_FAILURE" -R $owner/ockam
  # Sleep for 10 seconds to ensure we are not affected by Github API downtime.
  sleep 10
  # Wait for workflow run
  run_id=$(gh run list --workflow=publish_crates.yml -b develop -u $GITHUB_USERNAME -L 1 -R $owner/ockam --json databaseId | jq -r .[0].databaseId)
  gh run watch $run_id --exit-status -R $owner/ockam
}

function release_ockam_binaries() {
  gh workflow run release-binaries.yml --ref develop -F git_tag="$GIT_TAG" -R $owner/ockam
  # Wait for workflow run
  sleep 10
  run_id=$(gh run list --workflow=release-binaries.yml -b develop -u $GITHUB_USERNAME -L 1 -R $owner/ockam --json databaseId | jq -r .[0].databaseId)
  gh run watch $run_id --exit-status -R $owner/ockam
}

function homebrew_repo_bump() {
  gh workflow run create-release-pull-request.yml --ref main -F tag=$1 -R $owner/homebrew-ockam
  # Wait for workflow run
  sleep 10
  run_id=$(gh run list --workflow=create-release-pull-request.yml -b main -u $GITHUB_USERNAME -L 1 -R $owner/homebrew-ockam --json databaseId | jq -r .[0].databaseId)
  gh run watch $run_id --exit-status -R $owner/homebrew-ockam
}

function terraform_repo_bump() {
  gh workflow run create-release-pull-request.yml --ref main -R $owner/terraform-provider-ockam -F tag=$1
  # Wait for workflow run
  sleep 10
  run_id=$(gh run list --workflow=create-release-pull-request.yml -b main -u $GITHUB_USERNAME -L 1 -R $owner/terraform-provider-ockam  --json databaseId | jq -r .[0].databaseId)
  gh run watch $run_id --exit-status -R $owner/terraform-provider-ockam
}

function terraform_binaries_release() {
  gh workflow run release.yml --ref main -R $owner/terraform-provider-ockam -F tag=$1
  # Wait for workflow run
  sleep 10
  run_id=$(gh run list --workflow=release.yml -b main -u $GITHUB_USERNAME -L 1 -R $owner/terraform-provider-ockam  --json databaseId | jq -r .[0].databaseId)
  gh run watch $run_id --exit-status -R $owner/terraform-provider-ockam
}

function dialog_info() {
  echo -e "\033[01;33m$1\033[00m"
  read -p ""
}

function success_info() {
  echo -e "\033[01;32m$1\033[00m"
}

#------------------------------------------------------------------------------------------------------------------------------------------------------------------#

if [[ -z $SKIP_OCKAM_BUMP || $SKIP_OCKAM_BUMP == false ]]; then
  ockam_bump
  dialog_info "Crate bump pull request created.... Please merge pull request and press enter to start binaries release."
fi

if [[ -z $SKIP_CRATES_IO_PUBLISH || $SKIP_CRATES_IO_PUBLISH == false ]]; then
  ockam_crate_release
  success_info "Crates.io publish successful"
fi

if [[ -z $SKIP_OCKAM_BINARY_RELEASE || $SKIP_OCKAM_BINARY_RELEASE == false ]]; then
  release_ockam_binaries
  dialog_info "Draft release has been created, please vet and publish release then press enter to start homebrew and terraform CI"
  dialog_info "Script requires draft release to be published and tag created to accurately use latest tag.... Press enter if draft release has been published."
fi

# Get latest tag
if [[ -z $LATEST_TAG_NAME ]]; then
  latest_tag_name=$(curl -H "Accept: application/vnd.github.v3+json" https://api.github.com/repos/${owner}/ockam/releases/latest | jq -r .tag_name)
  dialog_info "Latest tag is $latest_tag_name press enter if correct"
else
  latest_tag_name=$LATEST_TAG_NAME
fi

# Homebrew Release
if [[ -z $SKIP_HOMEBREW_BUMP || $SKIP_HOMEBREW_BUMP == false ]]; then
  homebrew_repo_bump $latest_tag_name
  success_info "Homebrew bump successful."
fi

if [[ -z $SKIP_TERRAFORM_BUMP || $SKIP_TERRAFORM_BUMP == false ]]; then
  terraform_repo_bump $latest_tag_name
fi

dialog_info "Terraform pull request created, please vet and merge pull request then press enter to start Terraform binary release"

if [[ -z $SKIP_TERRAFORM_BINARY_RELEASE || $SKIP_TERRAFORM_BINARY_RELEASE == false ]]; then
  terraform_binaries_release $latest_tag_name
fi

success_info "Release Done 🚀🚀🚀"
