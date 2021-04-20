#!/usr/bin/env bash

pushd "$OCKAM_HOME/implementations/rust/ockam/ockam_examples/" >/dev/null || exit 1

for p in example_projects/*
do
  pushd "$p" >/dev/null || exit 1
  cargo build
  popd >/dev/null || exit 1
done

# TODO run more examples here

pushd example_projects/tcp >/dev/null || exit 1
cargo run --example network_echo_server &
SERVER=$!
sleep 2
cargo run --example network_echo_client
kill $SERVER

popd >/dev/null || exit 1

popd >/dev/null || exit 1
