# Examples directories, assuming pwd=implementations/python/tests/bats
if [[ -z $LOCAL_EXAMPLES_DIR ]]; then
  export LOCAL_EXAMPLES_DIR=../../examples
fi
if [[ -z $MAIN_EXAMPLES_DIR ]]; then
  export MAIN_EXAMPLES_DIR=../../../../examples
fi

function check_file_exists() {
  local file_path="$1"
  if [[ ! -f "$file_path" ]]; then
    echo "File $file_path does not exist"
    return 1
  fi
  return 0
}
function check_dir_exists() {
  local dir_path="$1"
  if [[ ! -d "$dir_path" ]]; then
    echo "Directory $dir_path does not exist"
    return 1
  fi
  return 0
}

function add_background_pid() {
  local pid="$1"
  if [[ -z "$PENDING_PIDS" ]]; then
    PENDING_PIDS="$pid"
  else
    PENDING_PIDS="$PENDING_PIDS;$pid"
  fi
}
function kill_background_pids() {
  local timeout=60
  local end=$(($(date +%s) + $timeout))

  if [[ -n "$PENDING_PIDS" ]]; then
    IFS=';' read -ra pids <<<"$PENDING_PIDS"

    # Send termination signal to all processes first
    for pid in "${pids[@]}"; do
      kill $pid 2>/dev/null || true
    done

    # Wait for processes to terminate
    local remaining_pids=("${pids[@]}")

    while [ ${#remaining_pids[@]} -gt 0 ] && [ $(date +%s) -lt $end ]; do
      # Create a new array to hold pids that are still running
      local still_running=()

      for pid in "${remaining_pids[@]}"; do
        if kill -0 $pid 2>/dev/null; then
          # Process still exists
          still_running+=("$pid")
        else
          # Process has terminated, wait for it to avoid zombies
          wait $pid 2>/dev/null || true
        fi
      done

      # Update the remaining_pids array
      remaining_pids=("${still_running[@]}")

      if [ ${#remaining_pids[@]} -eq 0 ]; then
        break
      fi

      sleep 1
    done

    # Calculate elapsed time for reporting
    local elapsed=$(($timeout - ($end - $(date +%s))))

    # Force kill any remaining processes
    for pid in "${remaining_pids[@]}"; do
      echo "Process $pid did not terminate after $elapsed seconds, force killing it" >&3
      kill -9 $pid 2>/dev/null || true
      wait $pid 2>/dev/null || true
    done

    unset PENDING_PIDS
  fi
}

function teardown_zone_test() {
  kill_background_pids
  $OCKAM zone delete --all --yes || true
}
