#!/usr/bin/env bash
set -ex

if [[ -z $GITHUB_USERNAME ]]; then
  echo "Please set your github username"
  exit 1
fi

if [[ -z $RELEASE_VERSION || $RELEASE_VERSION != *"ockam_v"* ]]; then
    echo "Please set RELEASE_VERSION variable, e.g. ockam_v0.63.0"
    exit 1
fi

owner="build-trust"

function test_published_crates_io_release() {
  set -e
  ockam_version=${RELEASE_VERSION:7}
  gh workflow run test_published_package.yml --ref docker -R $owner/artifacts -F ockam_version=$ockam_version

  # Wait for workflow run
  sleep 10
  run_id=$(gh run list --workflow=test_published_package.yml -b docker -u $GITHUB_USERNAME -L 1 -R $owner/artifacts  --json databaseId | jq -r .[0].databaseId)
  gh run watch $run_id --exit-status -R $owner/artifacts
}

test_published_crates_io_release