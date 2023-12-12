#!/usr/bin/env bash
set -e

# Pipe set -x log to a file https://stackoverflow.com/questions/25593034/capture-x-debug-commands-into-a-file-in-bash
log=$(mktemp)
echo "Log directory is $log"

exec 5>"$log"
BASH_XTRACEFD="5"

set -x

# Function to execute on exit
function on_exit() {
  echo "File log found here $log"
}

# Set trap to call on_exit function on script exit
trap on_exit EXIT

GITHUB_USERNAME=$(gh api user | jq -r '.login')

owner="build-trust"
release_name="release_$(date +'%d-%m-%Y')_$(date +'%s')"

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

function approve_deployment() {
  set -e
  local repository="$1"
  local run_id="$2"

  # completed waiting queued in_progress
  while true; do
    status=$(gh api -H "Accept: application/vnd.github+json" "/repos/$owner/$repository/actions/runs/$run_id" | jq -r '.status')
    if [[ $status == "completed" ]]; then
      echo "Run ID $run_id completed"
      return
    elif [[ $status == "waiting" ]]; then
      # Get actions that need to be approved
      pending_deployments=$(gh api -H "Accept: application/vnd.github+json" "/repos/$owner/$repository/actions/runs/$run_id/pending_deployments")
      pending_length=$(echo "$pending_deployments" | jq '. | length')

      environments=""
      for ((c = 0; c < pending_length; c++)); do
        environment=$(echo "$pending_deployments" | jq -r ".[$c].environment.id")
        environments="$environments $environment"
      done

      if [[ -n $environments ]]; then
        jq -n "{environment_ids: [$environments], state: \"approved\", comment: \"Ship It\"}" | gh api \
          --method POST \
          -H "Accept: application/vnd.github+json" \
          "/repos/$owner/$repository/actions/runs/$run_id/pending_deployments" --input - >> $log
      fi
    fi
    sleep 5
  done
}

function watch_workflow_progress() {
  set -e
  repository="$1"
  workflow_file_name="$2"
  branch="$3"

  echo "Waiting to workflow to be in progress before kickstarting workflow watcher"
  while true; do
    run_status=$(gh run list --workflow="$workflow_file_name" -b "$branch" -u "$GITHUB_USERNAME" -L 1 -R ${owner}/${repository} --json databaseId,status)
    status=$(jq -r '.[0].status' <<<$run_status)
    run_id=$(jq -r '.[0].databaseId' <<<$run_status)

    if [[ $status == 'in_progress' ]]; then
      gh run watch "$run_id" --exit-status -R ${owner}/${repository}
      return
    fi
  done
}

function approve_and_watch_workflow_progress() {
  set -e
  repository="$1"
  workflow_file_name="$2"
  branch="$3"

  run_id=$(gh run list --workflow="$workflow_file_name" -b "$branch" -u "$GITHUB_USERNAME" -L 1 -R ${owner}/${repository} --json databaseId | jq -r '.[0].databaseId')
  approve_deployment "$repository" "$run_id" &
  watch_workflow_progress "$repository" "$workflow_file_name"
}

function ockam_bump() {
  set -e
  workflow_file_name="release-bump-pull-request.yml"
  branch="develop"

  gh workflow run "$workflow_file_name" --ref "$branch" -F branch_name="$release_name" -F git_tag="$GIT_TAG" -F ockam_bump_modified_release="$OCKAM_BUMP_MODIFIED_RELEASE" \
    -F ockam_bump_release_version="$OCKAM_BUMP_RELEASE_VERSION" -F ockam_bump_bumped_dep_crates_version="$OCKAM_BUMP_BUMPED_DEP_CRATES_VERSION" \
    -R $owner/ockam >> $log

  # Sleep for 10 seconds to ensure we are not affected by Github API downtime.
  sleep 10
  # Wait for workflow run
  approve_and_watch_workflow_progress "ockam" "$workflow_file_name" "$branch"

  # Merge PR to a new branch to kickstart workflow
  gh pr create --title "Ockam Release $(date +'%d-%m-%Y')" --body "Ockam release" \
    --base "$branch" -H "${release_name}" -r mrinalwadhwa -R $owner/ockam >> $log
}

function ockam_crate_release() {
  set -e

  workflow_file_name="publish-crates.yml"
  branch="develop"

  gh workflow run "$workflow_file_name" --ref "$branch" \
    -F release_branch="$release_name" -F git_tag="$GIT_TAG" -F ockam_publish_exclude_crates="$OCKAM_PUBLISH_EXCLUDE_CRATES" \
    -F ockam_publish_recent_failure="$OCKAM_PUBLISH_RECENT_FAILURE" -R $owner/ockam >> $log
  # Sleep for 10 seconds to ensure we are not affected by Github API downtime.
  sleep 10
  approve_and_watch_workflow_progress "ockam" "$workflow_file_name" "$branch"
}

function release_ockam_binaries() {
  set -e
  workflow_file_name="release-draft-binaries.yml"
  branch="develop"

  gh workflow run "$workflow_file_name" --ref "$branch" -F git_tag="$GIT_TAG" -F release_branch="$release_name" -R $owner/ockam >> $log
  # Wait for workflow run
  sleep 10
  approve_and_watch_workflow_progress "ockam" "$workflow_file_name" "$branch"
}

function release_ockam_binaries_as_production() {
  set -e
  release_git_tag=$1
  workflow_name="release-production.yml"
  branch="develop"

  gh workflow run $workflow_name --ref "$branch" -F git_tag="$release_git_tag" -R $owner/ockam >> $log

  sleep 10
  approve_and_watch_workflow_progress "ockam" "$workflow_file_name" "$branch"
}

function release_ockam_package() {
  set -e
  tag="$1"
  file_and_sha="$2"
  is_release="$3"
  branch="develop"

  workflow_file_name="release-ockam-package.yml"

  gh workflow run "$workflow_file_name" --ref "$branch" -F tag="$tag" -F binaries_sha="$file_and_sha" -F is_release="$is_release" -R $owner/ockam >> $log
  # Wait for workflow run
  sleep 10
  approve_and_watch_workflow_progress "ockam" "$workflow_file_name" "$branch"
}

function homebrew_repo_bump() {
  set -e
  tag=$1
  file_and_sha=$2
  branch="main"

  workflow_file_name="release-bump-pull-request.yml"

  gh workflow run "$workflow_file_name" --ref "$branch" -F binaries="$file_and_sha" -F branch_name="$release_name" -F tag="$tag" -R $owner/homebrew-ockam >> $log
  # Wait for workflow run
  sleep 10
  approve_and_watch_workflow_progress "homebrew-ockam" "$workflow_file_name" "$branch"

  # Create PR to kickstart workflow
  gh pr create --title "Ockam Release $(date +'%d-%m-%Y')" --body "Ockam release" \
    --base main -H "${release_name}" -r mrinalwadhwa -R $owner/homebrew-ockam >> $log
}

function terraform_repo_bump() {
  set -e
  workflow_file_name="release-bump-pull-request.yml"
  branch="main"

  gh workflow run "$workflow_file_name" --ref "$branch" -R $owner/terraform-provider-ockam -F tag="$1" -F branch_name="$release_name" >> $log
  # Wait for workflow run
  sleep 10
  approve_and_watch_workflow_progress "terraform-provider-ockam" "$workflow_file_name" "$branch"

  # Create PR to kickstart workflow
  gh pr create --title "Ockam Release $(date +'%d-%m-%Y')" --body "Ockam release" \
    --base main -H "${release_name}" -r mrinalwadhwa -R $owner/terraform-provider-ockam >> $log
}

function terraform_binaries_release() {
  set -e
  workflow_file_name="release.yml"
  branch="main"

  gh workflow run "$workflow_file_name" --ref "$branch" -R $owner/terraform-provider-ockam -F tag="$1" >> $log
  # Wait for workflow run
  sleep 10

  approve_and_watch_workflow_progress "terraform-provider-ockam" "$workflow_file_name" "$branch"
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
          "/orgs/$owner/packages/container/ockam/versions/$id" --input - >> $log
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
  if [[ -z $SKIP_OCKAM_BUMP || $SKIP_OCKAM_BUMP == false ]]; then
    echo "Starting Ockam crate bump"
    ockam_bump
    success_info "Crate bump pull request created.... Releasing draft binaries."
  fi

  if [[ -z $SKIP_OCKAM_BINARY_RELEASE || $SKIP_OCKAM_BINARY_RELEASE == false ]]; then
    success_info "Releasing Ockam draft binaries"
    release_ockam_binaries
    success_info "Draft release has been created.... Starting Homebrew release."
  fi
fi

# Get latest tag
if [[ -z $LATEST_TAG_NAME ]]; then
  latest_tag_name=$(gh api -H "Accept: application/vnd.github+json" /repos/$owner/ockam/releases | jq -r '.[0].tag_name')
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
  gh release download "$latest_tag_name" -p sha256sums.txt -R $owner/ockam

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

# If we are to release to production, we check if all draft release was successful.
if [[ $IS_DRAFT_RELEASE == false ]]; then
  success_info "Vetting draft release...."

  # Check if the SHAsum file exists for the released binaries. We generate shasum file
  # after all binaries are released.
  if [[ -z $SKIP_OCKAM_DRAFT_RELEASE || $SKIP_OCKAM_DRAFT_RELEASE == false ]]; then
    gh release download "$latest_tag_name" -p sha256sums.txt -R $owner/ockam -O "$(mktemp -d)/sha256sums.txt"
  fi

  # Check if terraform SHAsum file exist in Terraform draft release.
  if [[ -z $SKIP_TERRAFORM_DRAFT_RELEASE || $SKIP_TERRAFORM_DRAFT_RELEASE == false ]]; then
    gh release download "$latest_tag_name" -p "**SHA256SUMS" -R $owner/terraform-provider-ockam -O "$(mktemp -d)/sha256sums.txt"
  fi

  # To release crates to crates.io, we need to ensure that there's a bump PR that's open.
  if [[ -z $SKIP_OCKAM_PACKAGE_RELEASE || $SKIP_OCKAM_PACKAGE_RELEASE == false ]]; then
    ockam_prs=$(gh api -H "Accept: application/vnd.github+json" /repos/${owner}/ockam/pulls)
    if [[ $ockam_prs != *"Ockam Release"* ]]; then
      echo "Could not find ockam release PR"
      exit 1
    fi
  fi

  # Check if there's an homebrew PR
  if [[ -z $SKIP_OCKAM_HOMEBREW_RELEASE || $SKIP_OCKAM_HOMEBREW_RELEASE == false ]]; then
    ockam_prs=$(gh api -H "Accept: application/vnd.github+json" /repos/${owner}/homebrew-ockam/pulls)
    if [[ $ockam_prs != *"Ockam Release"* ]]; then
      echo "Could not find homebrew release PR"
      exit 1
    fi
  fi

  success_info "All draft release asset have been created. /ockam and /homebrew-ockam PRs can be merged now for release."
  dialog_info "Press enter to start production release"
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
    gh release edit "$terraform_tag" --draft=false -R $owner/terraform-provider-ockam
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

  success_info "Release Done 🚀🚀🚀."
fi

exit 0
