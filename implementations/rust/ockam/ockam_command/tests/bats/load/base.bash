#!/bin/bash

# Ockam binary to use
if [[ -z $OCKAM ]]; then
  export OCKAM=ockam
fi

if [[ -z $BATS_LIB ]]; then
  BATS_LIB=$(brew --prefix)/lib # macos
  # BATS_LIB=$NVM_DIR/versions/node/v18.8.0/lib/node_modules # linux
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
      echo "pushd failed"
      exit 1
    }
    python3 -m http.server --bind 127.0.0.1 5000 &
    pid="$!"
    echo "$pid" >"$p"
    popd || {
      echo "popd failed"
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
  echo "$HOME/.ockam/http_server.pid"
}

# Sets the CLI directory to a random directory under /tmp
setup_home_dir() {
  dir="$BATS_FILE_TMPDIR/$(openssl rand -hex 8)"
  export OCKAM_HOME="$dir"
}

teardown_home_dir() {
  $OCKAM node delete --all --force
  $OCKAM reset -y
}

to_uppercase() {
  echo "$1" | tr 'a-z' 'A-Z'
}

# Returns a random name
random_str() {
  echo "$(openssl rand -hex 8)"
}

bats_require_minimum_version 1.5.0
