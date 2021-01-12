#!/bin/bash

USER_ID=${HOST_USER_ID:-9001}

useradd --shell /bin/bash -u $USER_ID -o -c "" -m runner
export HOME=/home/runner

exec /usr/local/bin/gosu runner "$@"
