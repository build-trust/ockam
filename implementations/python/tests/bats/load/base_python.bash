# Examples directories, assuming pwd=implementations/python
if [[ -z $LOCAL_EXAMPLES_DIR ]]; then
  export LOCAL_EXAMPLES_DIR=examples
fi
if [[ -z $MAIN_EXAMPLES_DIR ]]; then
  export MAIN_EXAMPLES_DIR=../../examples
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
  if [[ -n "$PENDING_PIDS" ]]; then
    IFS=';' read -ra pids <<<"$PENDING_PIDS"
    for pid in "${pids[@]}"; do
      kill $pid 2>/dev/null || true
      wait $pid 2>/dev/null || true
      sleep 5
      kill -9 $pid 2>/dev/null || true
    done
    unset PENDING_PIDS
  fi
}
