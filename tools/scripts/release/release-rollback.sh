#!/usr/bin/env bash
set -ex

# Pipe set -x log to a file https://stackoverflow.com/questions/25593034/capture-x-debug-commands-into-a-file-in-bash
log=$(mktemp)
echo "Log directory is $log"

exec 5>"$log"
BASH_XTRACEFD="5"

OWNER="metaclips"
USER_TYPE="users"

if [[ -z $TAG_NAME || $TAG_NAME != *"ockam_v"* ]]; then
  echo "Invalid tag name, please set TAG_NAME variable"
  exit 1
fi

function delete_release() {
  tag_name=$1
  repository=$2

  release_id=$(gh api \
    -H "Accept: application/vnd.github+json" \
    -H "X-GitHub-Api-Version: 2022-11-28" \
    /repos/$OWNER/$repository/releases/tags/$tag_name | jq '.id')

  if [[ $release_id == 'null' ]]; then
    echo "Draft release for $repository does not exist"
    return
  fi

  (gh api \
    --method DELETE \
    -H "Accept: application/vnd.github+json" \
    -H "X-GitHub-Api-Version: 2022-11-28" \
    /repos/$OWNER/$repository/releases/$release_id) || echo "error deleting draft release"
}

# Delete release in /ockam repository
delete_release $TAG_NAME "ockam"

# Delete Ockam tag
ockam_origin="https://github.com/$OWNER/ockam.git"
(git ls-remote --tags $ockam_origin | grep "$TAG_NAME" &>/dev/null && git push $ockam_origin --delete "$TAG_NAME") || echo "No tag created in /ockam, skipping."
# Delete Ockam release
(gh release list -R $OWNER/ockam | grep "$TAG_NAME" &>/dev/null && gh release delete "$TAG_NAME" -y -R $OWNER/ockam) || echo "No Ockam release created, skipping."

# Delete Terraform tag
terraform_tag_name=${TAG_NAME:6}

# Delete release in /ockam repository
delete_release $terraform_tag_name "terraform-provider-ockam"

terraform_origin="https://github.com/$OWNER/terraform-provider-ockam.git"
(git ls-remote --tags $terraform_origin | grep "$terraform_tag_name" &>/dev/null && git push $terraform_origin --delete "$terraform_tag_name") || echo "No tag created in /terraform-provider-ockam, skipping."
# Delete terraform release
(gh release list -R $OWNER/terraform-provider-ockam | grep "$terraform_tag_name" &>/dev/null && gh release delete "$terraform_tag_name" -y -R $OWNER/ockam) || echo "No /terraform-provider-ockam release created, skipping."

# Delete Ockam package
echo "Deleting packages"
versions=$(gh api -H "Accept: application/vnd.github+json" /$USER_TYPE/$OWNER/packages/container/ockam/versions)
echo "Deleting packages"
version_length=$(jq '. | length' <<<"$versions")
latest_tag=${TAG_NAME:7}

for ((c = 0; c < version_length; c++)); do
  id=$(jq -r ".[$c].id" <<<"$versions")

  tags=$(jq ".[$c].metadata.container.tags" <<<"$versions")
  tags_length=$(jq ". | length" <<<"$tags")

  for ((d = 0; d < tags_length; d++)); do
    tag_name=$(jq -r ".[$d]" <<<"$tags")

    if [[ $tag_name == "$latest_tag-draft" ]]; then
      echo "Deleting package with name $latest_tag-draft"
      echo -n | gh api \
        --method DELETE \
        -H "Accept: application/vnd.github+json" \
        /$USER_TYPE/$OWNER/packages/container/ockam/versions/"$id" --input -
      echo "Draft package deleted"
      break
    fi
  done
done

function close_pr() {
  set -e
  repository=$1

  ockam_prs=$(gh api -H "Accept: application/vnd.github+json" /repos/${OWNER}/"${repository}"/pulls)
  ockam_prs_length=$(jq '.|length' <<<"$ockam_prs")

  for ((c = 0; c < ockam_prs_length; c++)); do
    title=$(jq -r ".[$c].title" <<<"$ockam_prs")
    if [[ $title == *"Ockam Release"* ]]; then
      echo "closing PR in repository $repository with $title"
      pr_number=$(jq -r ".[$c].number" <<<"$ockam_prs")
      gh pr close "$pr_number" -R ${OWNER}/"${repository}"
      echo "PR closed"
      break
    fi
  done
}

close_pr "ockam"
close_pr "homebrew-ockam"
close_pr "terraform-provider-ockam"
