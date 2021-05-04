#!/usr/bin/env bash

source "$(dirname "$0")"/common.sh


change_dir "$OCKAM_RUST"
for d in *
do
  echo "$d $(crate_version "$d")"
done
pop_dir

