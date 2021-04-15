pushd "$OCKAM_HOME/implementations/rust/ockam/ockam_examples/example_projects" >/dev/null || exit 1

projects=(node worker)
for p in "${projects[@]}"
do
  pushd "$p" >/dev/null || exit 1
  for e in examples/*
  do
    EXAMPLE=$(basename "$e" .rs)
    cargo run --example "$EXAMPLE"
  done
  popd >/dev/null || exit 1
done

pushd tcp >/dev/null || exit 1
cargo run --example network_echo_server &
SERVER=$!
sleep 2
cargo run --example network_echo_client
kill $SERVER

popd >/dev/null || exit 1

popd >/dev/null || exit 1
