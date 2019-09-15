#!/bin/sh -eux

cd /tmp
download /opt/shellcheck "https://shellcheck.storage.googleapis.com/shellcheck-v0.6.0.linux.x86_64.tar.xz" \
  "95c7d6e8320d285a9f026b5241f48f1c02d225a1b08908660e8b84e58e9c7dce"

ln -s /opt/shellcheck/shellcheck /usr/bin/shellcheck
