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

function kill_ockam_pid() {
  if [[ -n "$OCKAM_PID" ]]; then
    kill $OCKAM_PID 2>/dev/null || true
    wait $OCKAM_PID 2>/dev/null || true
    unset OCKAM_PID
  fi
}
