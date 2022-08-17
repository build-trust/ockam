function release_ockam_binaries() {
  set -e
  gh workflow run release-binaries.yml --ref develop -F git_tag="$GIT_TAG" -F release_branch="$RELEASE_NAME" -R $OWNER/ockam
  # Wait for workflow run
  sleep 10
  run_id=$(gh run list --workflow=release-binaries.yml -b develop -u $GITHUB_USERNAME -L 1 -R $OWNER/ockam --json databaseId | jq -r .[0].databaseId)

  approve_deployment "ockam" $run_id &
  gh run watch $run_id --exit-status -R $OWNER/ockam
}
