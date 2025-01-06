#!/bin/bash

# Load bats extensions
load_bats_ext() {
  load "$BATS_LIB/bats-support/load.bash"
  load "$BATS_LIB/bats-assert/load.bash"
}

setup_python_server() {
  p=$(python_pid_file_path)
  if [[ ! -f "$p" ]]; then
    if ! [ -x "$(command -v uploadserver)" ]; then
      echo 'Error: uploadserver is not installed.' >&2
      exit 1
    fi

    # Create python server in the OCKAM_HOME_BASE directory
    pushd $OCKAM_HOME_BASE
    touch "$p"

    # Log server data to bats-tests directory
    uploadserver --bind 127.0.0.1 $PYTHON_SERVER_PORT &>"$HOME/.bats-tests/python_server.log" &
    pid="$!"
    echo "$pid" >"$p"
    popd || {
      exit 1
    }
  fi
}

teardown_python_server() {
  p=$(python_pid_file_path)
  if [[ -f "$p" ]]; then
    pid=$(cat "$p")
    kill -9 "$pid"
    rm "$p"
    wait "$pid" 2>/dev/null || true
  fi
}

python_pid_file_path() {
  echo "$OCKAM_HOME_BASE/.tmp/http_server.pid"
}

# Sets the CLI directory to a random directory under /tmp
setup_home_dir() {
  dir="$BATS_FILE_TMPDIR/$(openssl rand -hex 8)"
  mkdir -p $dir
  export OCKAM_HOME="$dir"
  if [[ -z "$HOME_DIRS" ]]; then
    export HOME_DIRS="$OCKAM_HOME"
  else
    export HOME_DIRS="$HOME_DIRS;$OCKAM_HOME"
  fi
}

mkdir -p "$HOME/.bats-tests"
teardown_home_dir() {
  IFS=';' read -ra DIRS <<<"$HOME_DIRS"
  for dir in "${DIRS[@]}"; do
    export OCKAM_HOME="$dir"
    # If BATS_TEST_COMPLETED is not set, the test failed.
    # If BATS_TEST_SKIPPED is not set, then the test was not skipped
    if [[ -z "$BATS_TEST_COMPLETED" && -z "$BATS_TEST_SKIPPED" ]]; then
      # Copy the CLI directory to $HOME/.bats-tests so it can be inspected.
      # For some reason, if the directory is moved, the teardown function gets stuck.
      echo "Failed test dir: $OCKAM_HOME" >&3
      cp -r "$OCKAM_HOME" "$HOME/.bats-tests"
    fi
    run $OCKAM node delete --all --yes
  done
  export OCKAM_HOME=$OCKAM_HOME_BASE
  run $OCKAM node delete --all --yes
}

force_kill_node() {
  max_retries=5
  i=0
  pid="$($OCKAM node show $1 --output json | jq .pid)"
  while [[ $i -lt $max_retries ]]; do
    run kill -9 $pid
    # Killing a background node (created without `-f`) leaves the
    # process in a defunct state when running within Docker.
    if ! ps -p $pid || ps -p $pid | grep defunct; then
      return
    fi
    sleep 0.2
    ((i = i + 1))
  done
}

to_uppercase() {
  echo "$1" | tr 'a-z' 'A-Z'
}

# Returns a random letter-only string
random_str() {
  echo "$(LC_ALL=C tr -dc 'a-zA-Z' </dev/urandom | head -c 10)"
}

# Returns a random hex string
random_hex_str() {
  echo "$(openssl rand -hex 4)"
}

# Returns a random port
random_port() {
  port=0
  max_retries=10
  i=0
  while [[ $i -lt $max_retries ]]; do
    port=$(shuf -i 10000-65535 -n 1 --random-source=/dev/urandom)
    netstat -latn -p tcp | grep $port >/dev/null
    if [[ $? == 1 ]]; then
      break
    fi
    ((i++))
    continue
  done
  if [ $i -eq $max_retries ]; then
    echo "Failed to find an open port" >&3
    exit 1
  fi
  echo "$port"
}

run_success() {
  run "$@"
  assert_success
}

run_failure() {
  run "$@"
  assert_failure
}

bats_require_minimum_version 1.5.0

# Disable the telemetry export to improve performances
export OCKAM_TELEMETRY_EXPORT=false

# Set a high timeout for CI tests
export OCKAM_DEFAULT_TIMEOUT=5m

# Set OCKAM_LOGGING to true so that command logs are persisted
export OCKAM_LOGGING=true

# Set QUIET to 1 to suppress user-facing logging written at stderr
export QUIET=true

# Ockam binary to use
if [[ -z $OCKAM ]]; then
  export OCKAM=ockam
fi

# Setup base directory for CLI state
if [[ -z $OCKAM_HOME ]]; then
  export OCKAM_HOME_BASE="$HOME/.ockam"
else
  export OCKAM_HOME_BASE="$OCKAM_HOME"
fi
if [ ! -d "$OCKAM_HOME_BASE" ]; then
  echo "Ockam CLI directory $OCKAM_HOME_BASE does not exist. Creating..." >&3
  mkdir -p "$OCKAM_HOME_BASE"
fi
mkdir -p "$OCKAM_HOME_BASE/.tmp"

if [[ -z $BATS_LIB ]]; then
  # macos
  if command -v brew 2>&1 >/dev/null; then
    export BATS_LIB=$(brew --prefix)/lib
  elif [[ -n $NVM_DIR ]]; then
    export BATS_LIB=$NVM_DIR/versions/node/v18.8.0/lib/node_modules # linux
  else
    export BATS_LIB=/usr/local/lib/node_modules # linux
  fi
fi

if [[ -z $PYTHON_SERVER_PORT ]]; then
  export PYTHON_SERVER_PORT=$(random_port)
fi

mkdir -p "$HOME/.bats-tests"
