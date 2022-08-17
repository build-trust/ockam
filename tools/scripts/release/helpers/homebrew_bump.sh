function homebrew_repo_bump() {
  set -e
  tag=$1
  file_and_sha=$2

  gh workflow run create-release-pull-request.yml --ref main -F binaries="$file_and_sha" -F branch_name="$RELEASE_NAME" -F tag=$tag -R $OWNER/homebrew-ockam
  # Wait for workflow run
  sleep 10
  run_id=$(gh run list --workflow=create-release-pull-request.yml -b main -u $GITHUB_USERNAME -L 1 -R $OWNER/homebrew-ockam --json databaseId | jq -r .[0].databaseId)

  approve_deployment "homebrew-ockam" $run_id &
  gh run watch $run_id --exit-status -R $OWNER/homebrew-ockam

  # Create PR to kickstart workflow
  gh pr create --title "Ockam Release $(date +'%d-%m-%Y')" --body "Ockam release"\
    --base main -H ${RELEASE_NAME} -r mrinalwadhwa -R $OWNER/homebrew-ockam
}
