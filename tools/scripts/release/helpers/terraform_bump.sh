function terraform_repo_bump() {
  set -e
  gh workflow run create-release-pull-request.yml --ref main -R $owner/terraform-provider-ockam -F tag=$1 -F branch_name=$release_name
  # Wait for workflow run
  sleep 10
  run_id=$(gh run list --workflow=create-release-pull-request.yml -b main -u $GITHUB_USERNAME -L 1 -R $owner/terraform-provider-ockam  --json databaseId | jq -r .[0].databaseId)

  approve_deployment "terraform-provider-ockam" $run_id &
  gh run watch $run_id --exit-status -R $owner/terraform-provider-ockam

  # Create PR to kickstart workflow
  gh pr create --title "Ockam Release $(date +'%d-%m-%Y')" --body "Ockam release"\
    --base main -H ${release_name} -r mrinalwadhwa -R $owner/terraform-provider-ockam
}
