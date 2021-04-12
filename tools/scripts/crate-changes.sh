#!/usr/bin/env bash

if [ -z "$OCKAM_ROOT" ]
then
  echo "Please set the OCKAM_ROOT environment variable to the ockam repository root directory."
  exit 0
fi

pushd $OCKAM_ROOT/implementations/rust/ockam/ >/dev/null
for d in *
do
  echo -n "git log ${d}_v"
  perl -ne '/^version = "([^"]+)"/ and print "$1..HEAD implementations/rust/ockam/"' < $d/Cargo.toml
  echo "$d >$d-change.log"
done
popd >/dev/null
