function release_ockam_package() {
  set -e
  tag="$1"
  file_and_sha="$2"
  is_release="$3"

  gh workflow run ockam-package.yml --ref develop -F tag="$tag" -F binaries_sha="$file_and_sha" -F is_release=$is_release  -R $OWNER/ockam
  # Wait for workflow run
  sleep 10
  run_id=$(gh run list --workflow=ockam-package.yml -b develop -u $GITHUB_USERNAME -L 1 -R $OWNER/ockam --json databaseId | jq -r .[0].databaseId)

  approve_deployment "ockam" $run_id &
  gh run watch $run_id --exit-status -R $OWNER/ockam
}
