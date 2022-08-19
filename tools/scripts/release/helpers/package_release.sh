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

function delete_ockam_draft_package() {
  set -e
  versions=$(gh api -H "Accept: application/vnd.github+json" /orgs/build-trust/packages/container/ockam/versions)
  version_length=$(echo "$versions" | jq '. | length')

  for (( c=0; c<$version_length; c++ )); do
    id=$(echo "$versions" | jq -r ".[$c].id")

    tags=$(echo "$versions" | jq ".[$c].metadata.container.tags")
    tags_length=$(echo "$tags" | jq ". | length")

    for (( d=0; d<$tags_length; d++ )); do
      tag_name=$(echo "$tags" | jq -r ".[$d]")

      if [[ $tag_name == *"-draft"* ]]; then
        echo -n | gh api \
          --method DELETE \
          -H "Accept: application/vnd.github+json" \
          /orgs/$OWNER/packages/container/ockam/versions/$id --input -
        break
      fi
    done
  done
}

function check_if_draft_package_exists() {

}

function check_if_production_package_is_released() {
  
}