#!/usr/bin/env bash
set -e

log=$(mktemp)
echo "Log directory is $log"

# Create a file descriptor that pipes stdout, stderr, and xtrace to a file and
# only logs stdout and stderr to terminal.
exec 3>"$log"
exec > >(tee -a /dev/fd/3) 2>&1

export BASH_XTRACEFD=3

set -x

# Function to execute on exit
function on_exit() {
  echo -e "\n\n=====> File log output can be found here $log" &>/dev/tty
  set +x
  unset BASH_XTRACEFD
}

# Set trap to call on_exit function on script exit
trap on_exit EXIT

GITHUB_USERNAME=$(gh api user | jq -r '.login')

if [[ -z $OWNER ]]; then
  OWNER="build-trust"
fi
release_name="release_$(date +'%d-%m-%Y')_$(date +'%s')"

if [[ -z $OCKAM_RELEASE_URL ]]; then
  echo "Please set Ockam AWS release URL e.g. https://ockam-release.s3.amazonaws.com"
  exit 1
fi

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

function failure_info() {
  echo -e "\033[01;31m$1\033[00m"
}

function dialog_info() {
  echo -e "\033[01;33m$1\033[00m"
  read -r -p ""
}

function success_info() {
  echo -e "\033[01;32m$1\033[00m"
}

function approve_deployment() {
  set -e
  local repository="$1"
  local run_id="$2"

  # completed waiting queued in_progress
  while true; do
    status=$(gh api -H "Accept: application/vnd.github+json" "/repos/$OWNER/$repository/actions/runs/$run_id" | jq -r '.status')
    if [[ $status == "completed" ]]; then
      echo "Run ID $run_id completed"
      return
    elif [[ $status == "waiting" ]]; then
      # Get actions that need to be approved
      pending_deployments=$(gh api -H "Accept: application/vnd.github+json" "/repos/$OWNER/$repository/actions/runs/$run_id/pending_deployments")
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
          "/repos/$OWNER/$repository/actions/runs/$run_id/pending_deployments" --input - >>$log
      fi
    fi
    sleep 5
  done
}

function watch_workflow_progress() {
  set -e
  repository="$1"
  workflow_file_name="$2"
  repo_branch="$3"

  echo "Waiting for workflow to be in progress before kickstarting gh run watch"
  while true; do
    run_status=$(gh run list --workflow="$workflow_file_name" -b "$repo_branch" -u "$GITHUB_USERNAME" -L 1 -R ${OWNER}/${repository} --json databaseId,status)
    status=$(jq -r '.[0].status' <<<$run_status)
    run_id=$(jq -r '.[0].databaseId' <<<$run_status)

    if [[ $status == 'in_progress' ]]; then
      gh run watch "$run_id" --exit-status -R ${OWNER}/${repository} &>/dev/tty
      return
    fi
  done
}

function approve_and_watch_workflow_progress() {
  set -e
  repository="$1"
  workflow_file_name="$2"
  repo_branch="$3"

  run_id=$(gh run list --workflow="$workflow_file_name" -b "$repo_branch" -u "$GITHUB_USERNAME" -L 1 -R ${OWNER}/${repository} --json databaseId | jq -r '.[0].databaseId')
  approve_deployment "$repository" "$run_id" &
  watch_workflow_progress "$repository" "$workflow_file_name" "$repo_branch"
}

function ockam_bump() {
  set -e
  workflow_file_name="release-bump-pull-request.yml"
  branch="develop"

  gh workflow run "$workflow_file_name" --ref "$branch" -F redo_release_tag="$REDO_RELEASE_TAG" -F branch_name="$release_name" -F git_tag="$GIT_TAG" -F ockam_bump_modified_release="$OCKAM_BUMP_MODIFIED_RELEASE" \
    -F ockam_bump_release_version="$OCKAM_BUMP_RELEASE_VERSION" -F ockam_bump_bumped_dep_crates_version="$OCKAM_BUMP_BUMPED_DEP_CRATES_VERSION" \
    -R $OWNER/ockam >>$log

  # Sleep for 10 seconds to ensure we are not affected by Github API downtime.
  sleep 10
  # Wait for workflow run
  approve_and_watch_workflow_progress "ockam" "$workflow_file_name" "$branch"

  # Merge PR to a new branch
  pr_link=$(gh pr create --title "Ockam Release $(date +'%d-%m-%Y')" --body "Ockam release" \
    --base "$branch" -H "${release_name}" -R $OWNER/ockam)

  # Wait for PR to be created
  success_info "Pull request that bumps ockam crates created ${pr_link}, please review and merge pull request to kickstart draft release..."

  while true; do
    state=$(gh pr view "$pr_link" --json state -R $OWNER/ockam | jq -r '.state')

    if [[ "$state" == "MERGED" ]]; then
      success_info "Pull request ${pr_link} merged, starting draft release..."
      break
    elif [[ "$state" == "OPEN" ]]; then
      sleep 5
      continue
    elif [[ "$state" == "CLOSED" ]]; then
      failure_info "Crate bump pull request was closed, aborting release."
      exit 1
    fi
  done
}

function ockam_crate_release() {
  set -e

  workflow_file_name="release-publish-crates.yml"
  branch="develop"
  GIT_TAG="$1"

  # Crate release from the develop branch
  gh workflow run "$workflow_file_name" --ref "$branch" \
    -F release_git_tag="$GIT_TAG" -F ockam_publish_exclude_crates="$OCKAM_PUBLISH_EXCLUDE_CRATES" \
    -F ockam_publish_recent_failure="$OCKAM_PUBLISH_RECENT_FAILURE" -R $OWNER/ockam >>$log
  # Sleep for 10 seconds to ensure we are not affected by Github API downtime.
  sleep 10
  approve_and_watch_workflow_progress "ockam" "$workflow_file_name" "$branch"
}

function release_ockam_binaries() {
  set -e
  workflow_file_name="release-draft-binaries.yml"
  branch="develop"

  gh workflow run "$workflow_file_name" --ref "$branch" -F git_tag="$GIT_TAG" -F release_branch="$branch" -R $OWNER/ockam >>$log
  # Wait for workflow run
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

  gh workflow run "$workflow_file_name" --ref "$branch" -F tag="$tag" -F binaries_sha="$file_and_sha" -F is_release="$is_release" -R $OWNER/ockam >>$log
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

  gh workflow run "$workflow_file_name" --ref "$branch" -F binaries="$file_and_sha" -F branch_name="$release_name" -F tag="$tag" -R $OWNER/homebrew-ockam >>$log
  # Wait for workflow run
  sleep 10
  approve_and_watch_workflow_progress "homebrew-ockam" "$workflow_file_name" "$branch"

  # Create PR to kickstart workflow
  gh pr create --title "Ockam Release $(date +'%d-%m-%Y')" --body "Ockam release" \
    --base main -H "${release_name}" -r mrinalwadhwa -R $OWNER/homebrew-ockam >>$log
}

function update_docs_repo() {
  set -e
  workflow_file_name="release-docs-update.yml"
  release_tag="$1"
  branch_name="main"

  gh workflow run "$workflow_file_name" --ref $branch_name -F branch_name="$release_name" -F ockam_ref="$release_tag" \
    -R $OWNER/ockam-documentation
  # Sleep for 10 seconds to ensure we are not affected by Github API downtime.
  sleep 10

  # Wait for workflow run
  approve_and_watch_workflow_progress "ockam-documentation" "$workflow_file_name" "$branch_name"

  # Check if the branch was created, new branch is only created when there are new doc updates
  if gh api "repos/build-trust/ockam-documentation/branches/docs_${release_name}" --jq .name; then
    gh pr create --title "Ockam Release $(date +'%d-%m-%Y')" --body "Ockam release" \
      --base main -H "docs_${release_name}" -r nazmulidris -R $OWNER/ockam-documentation
  fi
}

function update_command_manual() {
  set -e
  workflow_file_name="release-command-manual.yml"
  branch="main"
  release_tag="$1"

  prefix="ockam_"
  release=${release_tag#"$prefix"}

  gh workflow run "$workflow_file_name" --ref "$branch" -R $OWNER/ockam-documentation -F release_branch="$release_name" -F release_tag="$release" >>$log
  # Wait for workflow run
  sleep 10

  approve_and_watch_workflow_progress "ockam-documentation" "$workflow_file_name" "$branch"

  gh pr create --title "Ockam command manual update to $release" --body "Ockam commnad manual update $release" \
    --base command -H "manual_${release_name}" -R $OWNER/ockam-documentation >>$log
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
          "/orgs/$OWNER/packages/container/ockam/versions/$id" --input - >>$log
        break
      fi
    done
  done
}

#------------------------------------------------------------------------------------------------------------------------------------------------------------------#
# Perform Ockam bump and binary draft release if specified
if [[ $IS_DRAFT_RELEASE == true ]]; then
  if [[ -z $SKIP_OCKAM_BUMP || $SKIP_OCKAM_BUMP == false ]]; then
    echo "Starting Ockam crate bump"
    ockam_bump
    success_info "Crate bump pull request created...."
  fi

  if [[ -z $SKIP_OCKAM_BINARY_RELEASE || $SKIP_OCKAM_BINARY_RELEASE == false ]]; then
    success_info "Releasing Ockam draft binaries"
    release_ockam_binaries
    success_info "Draft release assets has been created...."
  fi
fi

# Get latest tag
if [[ -z $LATEST_TAG_NAME ]]; then
  latest_tag_name=$(gh release list -R "${OWNER}/ockam" --json tagName | jq -r '.[0].tagName')
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

  version="${latest_tag_name//ockam_/}"
  curl -O -R "${OCKAM_RELEASE_URL}/${version}/sha256sums.txt"

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

  echo "File and hash are $file_and_sha" >&3

  if [[ -z $SKIP_OCKAM_PACKAGE_RELEASE || $SKIP_OCKAM_PACKAGE_RELEASE == false ]]; then
    echo "Releasing Ockam docker image"
    release_ockam_package "$latest_tag_name" "$file_and_sha" false
    success_info "Ockam docker package draft release successful...."
  fi

  # Homebrew Release
  if [[ -z $SKIP_HOMEBREW_BUMP || $SKIP_HOMEBREW_BUMP == false ]]; then
    echo "Bumping Homebrew"
    homebrew_repo_bump "$latest_tag_name" "$file_and_sha"
    success_info "Homebrew release successful...."
  fi

  if [[ -z $SKIP_DOCS_UPDATE || $SKIP_DOCS_UPDATE == false ]]; then
    echo "Updating ockam documentation repository"
    update_docs_repo "$latest_tag_name"
    success_info "Ockam documentation repository pull request created..."
  fi

  if [[ -z $SKIP_COMMAND_MANUAL_RELEASE || $SKIP_COMMAND_MANUAL_RELEASE == false ]]; then
    echo "Updating ockam command manual"
    update_command_manual "$latest_tag_name"
    success_info "Ockam command manual updated successfully"
  fi

  success_info "Ockam draft release successful"
fi

# If we are to release to production, we check if all draft release was successful.
if [[ $IS_DRAFT_RELEASE == false ]]; then
  success_info "Checking if recent draft release was successful...."

  # Check if the SHAsum file exists for the released binaries. We generate shasum file
  # after all binaries are released.
  if [[ -z $SKIP_OCKAM_DRAFT_RELEASE || $SKIP_OCKAM_DRAFT_RELEASE == false ]]; then
    gh release download "$latest_tag_name" -p sha256sums.txt -R $OWNER/ockam -O "$(mktemp -d)/sha256sums.txt"
  fi

  # Check if there's an homebrew PR
  if [[ -z $SKIP_OCKAM_HOMEBREW_RELEASE || $SKIP_OCKAM_HOMEBREW_RELEASE == false ]]; then
    ockam_prs=$(gh api -H "Accept: application/vnd.github+json" /repos/${OWNER}/homebrew-ockam/pulls)
    if [[ $ockam_prs != *"Ockam Release"* ]]; then
      echo "Could not find homebrew release PR"
      exit 1
    fi
  fi

  # Check if ockam command manual PR was created
  if [[ -z $SKIP_COMMAND_MANUAL_RELEASE || $SKIP_COMMAND_MANUAL_RELEASE == false ]]; then
    ockam_prs=$(gh pr list --base command -R build-trust/ockam-documentation --json title)
    prefix="ockam_"
    release=${latest_tag_name#"$prefix"}
    if [[ $ockam_prs != *"$release"* ]]; then
      echo "Could not find command manual PR"
      exit 1
    fi
  fi

  success_info "Recent draft release was successful"
  dialog_info "Press enter if draft release text has been updated"

  dialog_info "Press enter to start final release"
fi

# Release to production
if [[ $IS_DRAFT_RELEASE == false ]]; then
  # Make Ockam Github draft as latest
  if [[ -z $SKIP_OCKAM_DRAFT_RELEASE || $SKIP_OCKAM_DRAFT_RELEASE == false ]]; then
    echo "Releasing Ockam Github release"
    gh release edit "$latest_tag_name" --latest=true --prerelease=false --draft=false -R "${OWNER}/ockam"
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
    ockam_crate_release "$latest_tag_name"
    success_info "Crates.io publish successful."
  fi

  success_info "Release Done 🚀🚀🚀."
fi

exit 0
