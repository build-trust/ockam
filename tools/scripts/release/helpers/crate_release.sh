function ockam_crate_release() {
  set -e
  gh workflow run publish-crates.yml --ref develop \
    -F release_branch="$RELEASE_NAME" -F git_tag="$GIT_TAG" -F ockam_publish_exclude_crates="$OCKAM_PUBLISH_EXCLUDE_CRATES" \
    -F ockam_publish_recent_failure="$OCKAM_PUBLISH_RECENT_FAILURE" -R $OWNER/ockam
  # Sleep for 10 seconds to ensure we are not affected by Github API downtime.
  sleep 10
  # Wait for workflow run
  run_id=$(gh run list --workflow=publish-crates.yml -b develop -u $GITHUB_USERNAME -L 1 -R $OWNER/ockam --json databaseId | jq -r .[0].databaseId)

  approve_deployment "ockam" $run_id &
  gh run watch $run_id --exit-status -R $OWNER/ockam
}
