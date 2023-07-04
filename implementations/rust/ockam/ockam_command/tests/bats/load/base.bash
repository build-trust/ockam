#!/bin/bash

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

if [[ -z $BATS_LIB ]]; then
  export BATS_LIB=$(brew --prefix)/lib # macos
  # export BATS_LIB=$NVM_DIR/versions/node/v18.8.0/lib/node_modules # linux
fi

# Load bats extensions
load_bats_ext() {
  load "$BATS_LIB/bats-support/load.bash"
  load "$BATS_LIB/bats-assert/load.bash"
}

setup_python_server() {
  p=$(python_pid_file_path)
  if [[ ! -f "$p" ]]; then
    mkdir -p "${p%/*}" && touch "$p"
    pushd "$(mktemp -d 2>/dev/null || mktemp -d -t 'tmpdir')" &>/dev/null || {
      exit 1
    }
    python3 -m http.server --bind 127.0.0.1 5000 &
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
  echo "$OCKAM_HOME_BASE/http_server.pid"
}

# Sets the CLI directory to a random directory under /tmp
setup_home_dir() {
  dir="$BATS_FILE_TMPDIR/$(openssl rand -hex 8)"
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
    OCKAM_HOME="$dir"
    # If BATS_TEST_COMPLETED is not set, the test failed.
    if [[ -z "$BATS_TEST_COMPLETED" ]]; then
      # Copy the CLI directory to $HOME/.bats-tests so it can be inspected.
      # For some reason, if the directory is moved, the teardown function gets stuck.
      echo "Failed test dir: $OCKAM_HOME" >&3
      cp -a "$OCKAM_HOME" "$HOME/.bats-tests"
    fi
    $OCKAM node delete --all --force
    $OCKAM reset -y
  done
}

to_uppercase() {
  echo "$1" | tr 'a-z' 'A-Z'
}

# Returns a random name
random_str() {
  echo "$(openssl rand -hex 4)"
}

# Returns a random port in the range 49152-65535
random_port() {
  port=0
  max_retries=10
  i=0
  while [[ $i -lt $max_retries ]]; do
    port=$(shuf -i 10000-65535 -n 1)
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

bats_require_minimum_version 1.5.0
