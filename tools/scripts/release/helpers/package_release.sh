function delete_if_draft_package_exists() {
  set -e
  latest_tag=$1
  latest_tag=${latest_tag:7}

  versions=$(gh api -H "Accept: application/vnd.github+json" /orgs/$OWNER/packages/container/ockam/versions)
  version_length=$(jq '. | length' <<< $versions)

  for (( c=0; c<$version_length; c++ )); do
    id=$(jq -r ".[$c].id" <<< $versions)

    tags=$(jq ".[$c].metadata.container.tags" <<< $versions)
    tags_length=$(jq ". | length" <<< $tags)

    for (( d=0; d<$tags_length; d++ )); do
      tag_name=$(jq -r ".[$d]" <<< $tags)

      if [[ $tag_name == "$latest_tag-draft" ]]; then
        echo -n | gh api \
          --method DELETE \
          -H "Accept: application/vnd.github+json" \
          /orgs/$OWNER/packages/container/ockam/versions/$id --input -
        break
      fi
    done
  done
}

function release_ockam_package() {
  set -e
  latest_tag=$1
  latest_tag=${latest_tag:7}

  versions=$(gh api -H "Accept: application/vnd.github+json" /orgs/$OWNER/packages/container/ockam/versions)
  version_length=$(jq '. | length' <<< $versions)

  for (( c=0; c<$version_length; c++ )); do
    id=$(jq -r ".[$c].id" <<< $versions)

    tags=$(jq ".[$c].metadata.container.tags" <<< $versions)
    tags_length=$(jq ". | length" <<< $tags)

    for (( d=0; d<$tags_length; d++ )); do
      tag_name=$(jq -r ".[$d]" <<< $tags)

      if [[ $tag_name == "$latest_tag" ]]; then
        return
      fi
    done
  done

  file_and_sha="$2"
  is_release="$3"

  gh workflow run ockam-package.yml --ref develop -F tag="$latest_tag" -F binaries_sha="$file_and_sha" -F is_release=$is_release  -R $OWNER/ockam
  # Wait for workflow run
  sleep 10
  run_id=$(gh run list --workflow=ockam-package.yml -b develop -u $GITHUB_USERNAME -L 1 -R $OWNER/ockam --json databaseId | jq -r .[0].databaseId)

  approve_deployment "ockam" $run_id &
  gh run watch $run_id --exit-status -R $OWNER/ockam
}