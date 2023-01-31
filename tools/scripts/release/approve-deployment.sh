if [[ -z $OWNER ]]; then
    echo "Please specify organization OWNER, e.g. build-trust"
    exit 1
fi

function approve_deployment() {
  set -e
  local repository="$1"
  local run_id="$2"

  # completed waiting queued in_progress
  while true; do
    status=$(gh api -H "Accept: application/vnd.github+json" "/repos/$OWNER/$repository/actions/runs/$run_id" | jq -r '.status')
    if [[ $status == "completed" ]]; then
      echo "Run ID $run_id completed"
      return
    elif [[ $status == "waiting" ]]; then
      # Get actions that need to be approved
      pending_deployments=$(gh api -H "Accept: application/vnd.github+json" "/repos/$OWNER/$repository/actions/runs/$run_id/pending_deployments")
      pending_length=$(echo "$pending_deployments" | jq '. | length')

      environments=""
      for ((c = 0; c < pending_length; c++)); do
        environment=$(echo "$pending_deployments" | jq -r ".[$c].environment.id")
        environments="$environments $environment"
      done

      if [[ -n $environments ]]; then
        jq -n "{environment_ids: [$environments], state: \"approved\", comment: \"Ship It\"}" | gh api \
          --method POST \
          -H "Accept: application/vnd.github+json" \
          "/repos/$OWNER/$repository/actions/runs/$run_id/pending_deployments" --input -
      fi
    fi
    sleep 120
  done
}
