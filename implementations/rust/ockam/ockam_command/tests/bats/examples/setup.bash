#!/bin/bash

wait_till_container_starts() {
  container_to_listen_to="$1"
  timeout=$2
  if [[ -z $timeout ]]; then
    timeout="250s"
  fi

  timeout "$timeout" bash <<EOT
    while true; do
      sleep 2
      docker logs "$container_to_listen_to" >/dev/null || continue
      break
    done
EOT
}

wait_till_successful_run_or_error() {
  container_to_listen_to="$1"
  # Wait till consumer exits and grab the exit code
  consumer_exit_code=$(docker wait "$container_to_listen_to")

  if [ "$consumer_exit_code" -eq 137 ]; then
    exit_code=0
    return
  fi

  exit_code=$consumer_exit_code
}

exit_on_successful() {
  container_to_listen_to="$1"
  while true; do
    logs=$(docker logs "$container_to_listen_to")
    if [[ "$logs" == *"The example run was successful 🥳."$'\n'* ]]; then
      docker stop "$container_to_listen_to"
      return
    fi
    sleep 1
  done
}

wait_till_pod_starts() {
  pod_to_listen_to="$1"
  timeout=$2
  if [[ -z $timeout ]]; then
    timeout="250s"
  fi

  timeout "$timeout" bash <<EOT
    while true; do
      sleep 2
      kubectl logs "$pod_to_listen_to" >/dev/null || continue
      break
    done
EOT
}
