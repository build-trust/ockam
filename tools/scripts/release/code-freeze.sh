#!/usr/bin/env bash
set -e

code_freeze_file_name="activate-code-freeze.yml"

if [[ -z $OWNER ]]; then
  echo "Please specify organization OWNER, e.g. build-trust"
  exit 1
fi

if [[ -z $IS_DRAFT_RELEASE ]]; then
  echo "Please set IS_DRAFT_RELEASE env to \`true\` or \`false\` if to release as \`draft\` or to \`production\`"
  exit 1
fi

code_freeze_branch_name="code_freeze_$(date +'%d-%m-%Y')_$(date +'%s')"

# Check PR to ensure it's merged
function check_pr_till_merge() {
  set -e
  pr_url=$1
  pr_state=$(check_pr_status $pr_url)
  echo "Code freeze PR $pr_url not merged, please merge PR to start release."

  while [[ $pr_state != 'MERGED' ]]; do
    if [[ $pr_state == 'CLOSED' ]]; then
      echo "PR was closed without merging... exiting script"
      exit 1
    fi

    sleep 5
    pr_state=$(check_pr_status $pr_url)
  done

  echo "PR $pr_url merge, kickstarting release."
}

function check_pr_status() {
  set -e
  pr_url=$1
  pr_state=$(gh pr view "$pr_url" --json state -R ${OWNER}/ockam | jq -r '.state')
  echo "$pr_state"
}

function start_code_freeze_workflow() {
  set -e
  freeze_state="$1"

  gh workflow run $code_freeze_file_name --ref develop \
    -F branch_name="$code_freeze_branch_name" -F set_freeze_state="$freeze_state" -R $OWNER/ockam

  # Sleep for 10 seconds to ensure we are not affected by Github API downtime.
  sleep 10

  # Wait for workflow run
  run_id=$(gh run list --workflow=$code_freeze_file_name -b develop -u "$GITHUB_USERNAME" -L 1 -R $OWNER/ockam --json databaseId | jq -r .[0].databaseId)
  approve_deployment "ockam" "$run_id" &

  gh run watch "$run_id" --exit-status -R $OWNER/ockam
}

function create_pr() {
  pr_title="$1"
  body="$2"

  # Merge PR to a new branch to kickstart workflow
  pr_url=$(gh pr create --title "$pr_title" --body "$body" \
    --base develop -H "${code_freeze_branch_name}" -r mrinalwadhwa -R $OWNER/ockam)

  check_pr_till_merge $pr_url
}

# If it's a draft release, then we create a PR that perform code freeze
if [[ $IS_DRAFT_RELEASE == true && $SKIP_CODE_FREEZE != true ]]; then
  echo "Kickstarting code freeze workflow"
  start_code_freeze_workflow "freeze"
  echo "Creating PR"
  create_pr "Ockam code freeze $(date +'%d-%m-%Y')" "This PR freezes Rust code PR merge."
elif [[ $IS_DRAFT_RELEASE == false && $SKIP_CODE_FREEZE != true ]]; then
  echo "Kickstarting code unfreeze workflow"
  start_code_freeze_workflow "unfreeze"
  echo "Creating PR"
  create_pr "Ockam code unfreeze $(date +'%d-%m-%Y')" "This PR unfreezes Rust code PR merge."
fi
