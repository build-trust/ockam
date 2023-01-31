#!/usr/bin/env bash
set -e

# Pipe set -x log to a file https://stackoverflow.com/questions/25593034/capture-x-debug-commands-into-a-file-in-bash
log=$(mktemp)
echo "Log directory is $log"

exec 5>"$log"
BASH_XTRACEFD="5"

set -x

GITHUB_USERNAME=$(gh api user | jq -r '.login')

OWNER="build-trust"
release_name="release_$(date +'%d-%m-%Y')_$(date +'%s')"

source tools/scripts/release/approve-deployment.sh

# Ensure all executables are installed
executables_installed=true
if ! command -v jq &>/dev/null; then
  echo "JQ executable not installed. Please install at https://stedolan.github.io/jq/"
  executables_installed=false
fi
if ! command -v gh &>/dev/null; then
  echo "Github CLI not installed. Please install at https://cli.github.com"
  executables_installed=false
fi

if [[ $executables_installed == false ]]; then
  echo "Required executables not installed. Exiting now."
  exit 1
fi

if [[ -z $OCKAM_PUBLISH_RECENT_FAILURE ]]; then
  OCKAM_PUBLISH_RECENT_FAILURE=false
fi

if [[ -z $IS_DRAFT_RELEASE ]]; then
  echo "Please set IS_DRAFT_RELEASE env to \`true\` or \`false\` if to release as \`draft\` or to \`production\`"
  exit 1
fi


function ockam_bump() {
  set -e
  gh workflow run create-release-pull-request.yml --ref develop -F branch_name="$release_name" -F git_tag="$GIT_TAG" -F ockam_bump_modified_release="$OCKAM_BUMP_MODIFIED_RELEASE" \
    -F ockam_bump_release_version="$OCKAM_BUMP_RELEASE_VERSION" -F ockam_bump_bumped_dep_crates_version="$OCKAM_BUMP_BUMPED_DEP_CRATES_VERSION" \
    -R $OWNER/ockam

  workflow_file_name="create-release-pull-request.yml"
  # Sleep for 10 seconds to ensure we are not affected by Github API downtime.
  sleep 10
  # Wait for workflow run
  run_id=$(gh run list --workflow="$workflow_file_name" -b develop -u "$GITHUB_USERNAME" -L 1 -R $OWNER/ockam --json databaseId | jq -r .[0].databaseId)

  approve_deployment "ockam" "$run_id" &
  gh run watch "$run_id" --exit-status -R $OWNER/ockam

  # Merge PR to a new branch to kickstart workflow
  gh pr create --title "Ockam Release $(date +'%d-%m-%Y')" --body "Ockam release" \
    --base develop -H "${release_name}" -r mrinalwadhwa -R $OWNER/ockam
}

function ockam_crate_release() {
  set -e
  gh workflow run publish-crates.yml --ref develop \
    -F release_branch="$release_name" -F git_tag="$GIT_TAG" -F ockam_publish_exclude_crates="$OCKAM_PUBLISH_EXCLUDE_CRATES" \
    -F ockam_publish_recent_failure="$OCKAM_PUBLISH_RECENT_FAILURE" -R $OWNER/ockam
  # Sleep for 10 seconds to ensure we are not affected by Github API downtime.
  sleep 10
  # Wait for workflow run
  run_id=$(gh run list --workflow=publish-crates.yml -b develop -u "$GITHUB_USERNAME" -L 1 -R $OWNER/ockam --json databaseId | jq -r .[0].databaseId)

  approve_deployment "ockam" "$run_id" &
  gh run watch "$run_id" --exit-status -R $OWNER/ockam
}

function release_ockam_binaries() {
  set -e
  gh workflow run release-binaries.yml --ref develop -F git_tag="$GIT_TAG" -F release_branch="$release_name" -R $OWNER/ockam
  # Wait for workflow run
  sleep 10
  run_id=$(gh run list --workflow=release-binaries.yml -b develop -u "$GITHUB_USERNAME" -L 1 -R $OWNER/ockam --json databaseId | jq -r .[0].databaseId)

  approve_deployment "ockam" "$run_id" &
  gh run watch "$run_id" --exit-status -R $OWNER/ockam
}

function release_ockam_binaries_as_production() {
  set -e
  release_git_tag=$1
  workflow_name="release-tag.yml"

  gh workflow run $workflow_name --ref develop -F git_tag="$release_git_tag" -R $OWNER/ockam

  sleep 10
  run_id=$(gh run list --workflow="$workflow_name" -b develop -u "$GITHUB_USERNAME" -L 1 -R $OWNER/ockam --json databaseId | jq -r .[0].databaseId)

  approve_deployment "ockam" "$run_id" &
  gh run watch "$run_id" --exit-status -R $OWNER/ockam
}

function release_ockam_package() {
  set -e
  tag="$1"
  file_and_sha="$2"
  is_release="$3"

  gh workflow run ockam-package.yml --ref develop -F tag="$tag" -F binaries_sha="$file_and_sha" -F is_release="$is_release" -R $OWNER/ockam
  # Wait for workflow run
  sleep 10
  run_id=$(gh run list --workflow=ockam-package.yml -b develop -u "$GITHUB_USERNAME" -L 1 -R $OWNER/ockam --json databaseId | jq -r .[0].databaseId)

  approve_deployment "ockam" "$run_id" &
  gh run watch "$run_id" --exit-status -R $OWNER/ockam
}

function homebrew_repo_bump() {
  set -e
  tag=$1
  file_and_sha=$2

  gh workflow run create-release-pull-request.yml --ref main -F binaries="$file_and_sha" -F branch_name="$release_name" -F tag="$tag" -R $OWNER/homebrew-ockam
  # Wait for workflow run
  sleep 10
  run_id=$(gh run list --workflow=create-release-pull-request.yml -b main -u "$GITHUB_USERNAME" -L 1 -R $OWNER/homebrew-ockam --json databaseId | jq -r .[0].databaseId)

  approve_deployment "homebrew-ockam" "$run_id" &
  gh run watch "$run_id" --exit-status -R $OWNER/homebrew-ockam

  # Create PR to kickstart workflow
  gh pr create --title "Ockam Release $(date +'%d-%m-%Y')" --body "Ockam release" \
    --base main -H "${release_name}" -r mrinalwadhwa -R $OWNER/homebrew-ockam
}

function terraform_repo_bump() {
  set -e
  gh workflow run create-release-pull-request.yml --ref main -R $OWNER/terraform-provider-ockam -F tag="$1" -F branch_name="$release_name"
  # Wait for workflow run
  sleep 10
  run_id=$(gh run list --workflow=create-release-pull-request.yml -b main -u "$GITHUB_USERNAME" -L 1 -R $OWNER/terraform-provider-ockam --json databaseId | jq -r .[0].databaseId)

  approve_deployment "terraform-provider-ockam" "$run_id" &
  gh run watch "$run_id" --exit-status -R $OWNER/terraform-provider-ockam

  # Create PR to kickstart workflow
  gh pr create --title "Ockam Release $(date +'%d-%m-%Y')" --body "Ockam release" \
    --base main -H "${release_name}" -r mrinalwadhwa -R $OWNER/terraform-provider-ockam
}

function terraform_binaries_release() {
  set -e
  gh workflow run release.yml --ref main -R $OWNER/terraform-provider-ockam -F tag="$1"
  # Wait for workflow run
  sleep 10
  run_id=$(gh run list --workflow=release.yml -b main -u "$GITHUB_USERNAME" -L 1 -R $OWNER/terraform-provider-ockam --json databaseId | jq -r .[0].databaseId)

  approve_deployment "terraform-provider-ockam" "$run_id" &
  gh run watch "$run_id" --exit-status -R $OWNER/terraform-provider-ockam
}

function delete_ockam_draft_package() {
  set -e
  versions=$(gh api -H "Accept: application/vnd.github+json" /orgs/build-trust/packages/container/ockam/versions)
  version_length=$(echo "$versions" | jq '. | length')

  for ((c = 0; c < version_length; c++)); do
    id=$(echo "$versions" | jq -r ".[$c].id")

    tags=$(echo "$versions" | jq ".[$c].metadata.container.tags")
    tags_length=$(echo "$tags" | jq ". | length")

    for ((d = 0; d < tags_length; d++)); do
      tag_name=$(echo "$tags" | jq -r ".[$d]")

      if [[ $tag_name == *"-draft"* ]]; then
        echo -n | gh api \
          --method DELETE \
          -H "Accept: application/vnd.github+json" \
          "/orgs/$OWNER/packages/container/ockam/versions/$id" --input -
        break
      fi
    done
  done
}

function dialog_info() {
  echo -e "\033[01;33m$1\033[00m"
  read -r -p ""
}

function success_info() {
  echo -e "\033[01;32m$1\033[00m"
}

#------------------------------------------------------------------------------------------------------------------------------------------------------------------#
# Perform Ockam bump and binary draft release if specified
if [[ $IS_DRAFT_RELEASE == true ]]; then
  # Run code freeze script before any draft action is made
  source tools/scripts/release/code-freeze.sh

  if [[ -z $SKIP_OCKAM_BUMP || $SKIP_OCKAM_BUMP == false ]]; then
    echo "Starting Ockam crate bump"
    ockam_bump
    success_info "Crate bump pull request created.... Starting Ockam crates.io publish."
  fi

  if [[ -z $SKIP_OCKAM_BINARY_RELEASE || $SKIP_OCKAM_BINARY_RELEASE == false ]]; then
    echo "Releasing Ockam draft binaries"
    release_ockam_binaries
    success_info "Draft release has been created.... Starting Homebrew release."
  fi
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

if [[ $IS_DRAFT_RELEASE == true ]]; then
  # Get File hash from draft release
  echo "Retrieving Ockam file SHA"
  file_and_sha=""

  temp_dir=$(mktemp -d)
  pushd "$temp_dir"
  gh release download "$latest_tag_name" -R $OWNER/ockam

  # TODO Ensure that SHA are cosign verified
  while read -r line; do
    IFS=" " read -r -a file <<<"$line"
    if [[ ${file[1]} == *".so"* || ${file[1]} == *".sig"* ]]; then
      continue
    fi

    file_and_sha="$file_and_sha ${file[1]}:${file[0]}"
  done <sha256sums.txt
  popd
  rm -rf "/tmp/$release_name"

  echo "File and hash are $file_and_sha"

  if [[ -z $SKIP_OCKAM_PACKAGE_DRAFT_RELEASE || $SKIP_OCKAM_PACKAGE_DRAFT_RELEASE == false ]]; then
    echo "Releasing Ockam container image"
    release_ockam_package "$latest_tag_name" "$file_and_sha" false
    success_info "Ockam package draft release successful.... Starting Homebrew release"
  fi

  # Homebrew Release
  if [[ -z $SKIP_HOMEBREW_BUMP || $SKIP_HOMEBREW_BUMP == false ]]; then
    echo "Bumping Homebrew"
    homebrew_repo_bump "$latest_tag_name" "$file_and_sha"
    success_info "Homebrew release successful.... Starting Terraform Release"
  fi

  if [[ -z $SKIP_TERRAFORM_BUMP || $SKIP_TERRAFORM_BUMP == false ]]; then
    echo "Bumping Terraform"
    terraform_repo_bump "$latest_tag_name"
  fi

  if [[ -z $SKIP_TERRAFORM_BINARY_RELEASE || $SKIP_TERRAFORM_BINARY_RELEASE == false ]]; then
    echo "Releasing Ockam Terraform binaries"
    terraform_binaries_release "$latest_tag_name"
  fi

  success_info "Terraform release successful"
fi

# Release to production
if [[ $IS_DRAFT_RELEASE == false ]]; then
  # Make Ockam Github draft as latest
  if [[ -z $SKIP_OCKAM_DRAFT_RELEASE || $SKIP_OCKAM_DRAFT_RELEASE == false ]]; then
    echo "Releasing Ockam Github release"
    release_ockam_binaries_as_production "$latest_tag_name"
  fi

  # Release Terraform Github release
  if [[ -z $SKIP_TERRAFORM_DRAFT_RELEASE || $SKIP_TERRAFORM_DRAFT_RELEASE == false ]]; then
    echo "Releasing Terraform release"
    terraform_tag=${latest_tag_name:6}
    gh release edit "$terraform_tag" --draft=false -R $OWNER/terraform-provider-ockam
  fi

  # Release Ockam package
  if [[ -z $SKIP_OCKAM_PACKAGE_RELEASE || $SKIP_OCKAM_PACKAGE_RELEASE == false ]]; then
    echo "Making Ockam container latest"
    release_ockam_package "$latest_tag_name" "nil" true
    delete_ockam_draft_package
    success_info "Ockam package release successful."
  fi

  if [[ -z $SKIP_CRATES_IO_PUBLISH || $SKIP_CRATES_IO_PUBLISH == false ]]; then
    echo "Starting Crates IO publish"
    ockam_crate_release
    success_info "Crates.io publish successful."
  fi

  success_info "Release Done.... Unfreezing the Ockam repository"

  # Run code unfreeze script before after production is done
  source tools/scripts/release/code-freeze.sh
  success_info "Code unfreeze successful 🚀🚀🚀"
fi

exit 0
