function terraform_binaries_release() {
  set -e
  gh workflow run release.yml --ref main -R $OWNER/terraform-provider-ockam -F tag=$1
  # Wait for workflow run
  sleep 10
  run_id=$(gh run list --workflow=release.yml -b main -u $GITHUB_USERNAME -L 1 -R $OWNER/terraform-provider-ockam  --json databaseId | jq -r .[0].databaseId)

  approve_deployment "terraform-provider-ockam" $run_id &
  gh run watch $run_id --exit-status -R $OWNER/terraform-provider-ockam
}
