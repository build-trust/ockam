#!/usr/bin/env bash
set -ex

# Pipe set -x log to a file https://stackoverflow.com/questions/25593034/capture-x-debug-commands-into-a-file-in-bash
log=$(mktemp)
echo "Log directory is $log"

exec &>>"$log"

if [[ -z $OWNER ]]; then
  OWNER="build-trust"
fi

USER_TYPE="users"

user_type=$(gh api \
  -H "Accept: application/vnd.github+json" \
  -H "X-GitHub-Api-Version: 2022-11-28" \
  "/users/${OWNER}" | jq -r '.type')

if [[ "$user_type" == "Organization" ]]; then
  USER_TYPE="orgs"
fi

if [[ -z $TAG_NAME || $TAG_NAME != *"ockam_v"* ]]; then
  echo "Invalid tag name, please set TAG_NAME variable"
  exit 1
fi

function delete_release() {
  tag_name=$1
  repository=$2

  echo "Deleting release in $repository with tag $tag_name"
  gh release delete "$tag_name" -y --cleanup-tag -R "${OWNER}/${repository}"
}

function close_pr() {
  set -e
  repository=$1

  echo "Closing release PR in $repository"

  pull_request=$(gh pr list --search "Ockam Release " -R ${OWNER}/${repository} --state open --json title,state,number)
  length=$(jq '. | length' <<<$pull_request)
  echo "$length PRs seen"

  if [[ $length == 0 ]]; then
    echo "No pull request created"
    return
  fi

  for ((c = 0; c < $length; c++)); do
    pr_title=$(jq -r ".[${c}].title" <<<$pull_request)
    pr_state=$(jq -r ".[${c}].state" <<<$pull_request)
    pr_number=$(jq -r ".[${c}].number" <<<$pull_request)

    if [[ $pr_title == *"Ockam Release"* ]]; then
      echo "closing PR in repository $repository with $pr_title"
      gh pr close "$pr_number" -R ${OWNER}/"${repository}"
      echo "PR $pr_title closed"
    fi
  done
}

function delete_ockam_package() {
  # Delete Ockam package
  echo "Deleting packages"
  versions=$(gh api -H "Accept: application/vnd.github+json" /${USER_TYPE}/${OWNER}/packages/container/ockam/versions)
  echo "Deleting packages"
  version_length=$(jq '. | length' <<<"$versions")
  latest_tag=${TAG_NAME:7}

  for ((c = 0; c < $version_length; c++)); do
    id=$(jq -r ".[$c].id" <<<"$versions")

    tags=$(jq ".[$c].metadata.container.tags" <<<"$versions")
    tags_length=$(jq ". | length" <<<"$tags")

    for ((d = 0; d < tags_length; d++)); do
      tag_name=$(jq -r ".[$d]" <<<"$tags")

      if [[ $tag_name == "${latest_tag}-draft" ]]; then
        echo "Deleting package with name $latest_tag-draft"
        echo -n | gh api \
          --method DELETE \
          -H "Accept: application/vnd.github+json" \
          "/${USER_TYPE}/${OWNER}/packages/container/ockam/versions/$id" --input -

        echo "Draft package deleted"
        break
      fi
    done
  done
}

function fail_if_release_is_already_in_production() {
  echo "Checking if release is a draft release"
  is_draft=$(gh release view $TAG_NAME -R ${OWNER}/ockam --json isDraft | jq '.isDraft')
  if [[ $is_draft == 'false' ]]; then
    echo "Tag name $TAG_NAME does not exist"
    exit 1
  fi
}

fail_if_release_is_already_in_production

# Delete release in /ockam repository
delete_release $TAG_NAME "ockam" || echo "No Ockam release created, skipping."

# Delete release in /terraform-provider-ockam repository
delete_release ${TAG_NAME:6} "terraform-provider-ockam" || echo "No Terraform release created, skipping."

delete_ockam_package || echo "Ockam draft package not created"

close_pr "ockam" || echo "Ockam PR not created"
close_pr "homebrew-ockam" || echo "Homebrew PR not created"
close_pr "terraform-provider-ockam" || echo "Terraform PR not created"
