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
