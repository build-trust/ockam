function ockam_bump() {
  set -e
  gh workflow run create-release-pull-request.yml --ref develop\
    -F branch_name="$RELEASE_NAME" -F git_tag="$GIT_TAG" -F ockam_bump_modified_release="$OCKAM_BUMP_MODIFIED_RELEASE"\
    -F ockam_bump_release_version="$OCKAM_BUMP_RELEASE_VERSION" -F ockam_bump_bumped_dep_crates_version="$OCKAM_BUMP_BUMPED_DEP_CRATES_VERSION"\
    -R $OWNER/ockam

  workflow_file_name="create-release-pull-request.yml"
  # Sleep for 10 seconds to ensure we are not affected by Github API downtime.
  sleep 10
  # Wait for workflow run
  run_id=$(gh run list --workflow="$workflow_file_name" -b develop -u $GITHUB_USERNAME -L 1 -R $OWNER/ockam --json databaseId | jq -r .[0].databaseId)

  approve_deployment "ockam" $run_id &
  gh run watch $run_id --exit-status -R $OWNER/ockam

  # Merge PR to a new branch to kickstart workflow
  gh pr create --title "Ockam Release $(date +'%d-%m-%Y')" --body "Ockam release"\
    --base develop -H ${RELEASE_NAME} -r mrinalwadhwa -R $OWNER/ockam
}
