#!/usr/bin/env bash
set -e

# Pipe set -x log to a file https://stackoverflow.com/questions/25593034/capture-x-debug-commands-into-a-file-in-bash
log=$(mktemp)
echo "Log directory is $log"

exec 5>$log
BASH_XTRACEFD="5"

set -x

GITHUB_USERNAME=$(gh api user | jq -r '.login')
owner="build-trust"

if [[ ! -z $RELEASE_VERSION && $RELEASE_VERSION != *"ockam_v"* ]]; then
  echo "Please set RELEASE_VERSION variable, e.g. ockam_v0.63.0"
  exit 1
fi

if [[ -z $RELEASE_VERSION ]]; then
  echo "Getting latest release"
  latest_tag_name=$(gh api -H "Accept: application/vnd.github+json" /repos/$owner/ockam/releases | jq -r .[0].tag_name)
  RELEASE_VERSION=$latest_tag_name
fi

function test_published_crates_io_release() {
  set -e
  ockam_version=${RELEASE_VERSION}
  gh workflow run acceptance_test.yml --ref docker -R $owner/artifacts -F ockam_version=$ockam_version

  # Wait for workflow run
  sleep 10
  run_id=$(gh run list --workflow=acceptance_test.yml -b docker -u $GITHUB_USERNAME -L 1 -R $owner/artifacts --json databaseId | jq -r .[0].databaseId)
  gh run watch $run_id --exit-status -R $owner/artifacts
}

test_published_crates_io_release
