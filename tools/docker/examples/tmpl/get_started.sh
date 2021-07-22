#!/usr/bin/env bash

# Requires an OCKAM_HUB environment variable. Set to a test instance.
if [ -z "$OCKAM_HUB" ]
then
  echo "Missing OCKAM_HUB environment variable. You must set OCKAM_HUB to a test Hub instance."
  exit 0
fi



# Get Started Examples
pushd $OCKAM_HOME >/dev/null || exit 1
pushd examples/rust/get_started >/dev/null || exit 1

# Build all examples
for e in examples/*.rs
do
  cargo build --example "$(basename "$e" .rs)" || exit 2
done

# Run examples that don't depend on a server: 1 through 6
local_examples=(01-node 02-worker 03-routing 04-routing-many-hops 05-secure-channel 06-secure-channel-many-hops)
for e in "${local_examples[@]}"
do
  cargo run --example "$e" || exit 2
done

# Step 7
cargo run --example 07-routing-over-transport-responder &
RESPONDER=$!
sleep 1
cargo run --example 07-routing-over-transport-initiator
kill $RESPONDER

# Step 8
cargo run --example 08-routing-over-transport-many-hops-responder &
RESPONDER=$!
sleep 1
cargo run --example 08-routing-over-transport-many-hops-middle &
MIDDLE=$!
sleep 1
cargo run --example 08-routing-over-transport-many-hops-initiator
kill $MIDDLE
kill $RESPONDER

# Step 9
cargo run --example 09-secure-channel-over-many-transport-hops-responder &
RESPONDER=$!
sleep 1
cargo run --example 09-secure-channel-over-many-transport-hops-middle &
MIDDLE=$!
sleep 1
cargo run --example 09-secure-channel-over-many-transport-hops-initiator
kill $MIDDLE
kill $RESPONDER

# Rewrite the "Paste" instructions to the hub instance.
for h in examples/*.rs
do
  EXAMPLE=$(basename "$h" .rs)
  perl -pe 's/^(.*?")Paste.*node here./${1}$ENV{"OCKAM_HUB"}/' < "$h" > "examples/my-$EXAMPLE.rs"
done

cargo run --example my-10-routing-to-a-hub-node

# Step 11
cargo run --example my-11-connecting-devices-using-hub-node-responder &>responder-11 &
RESPONDER=$!
echo "Waiting 10 seconds for forwarding address.."
sleep 10

# Read the address out of responder output.
export ADDRESS=$(perl -ne 'm/[fF]orwarding.*?: (\S+)$/ and print "$1\n"' responder-11)

# Rewrite the "Paste" instructions for forwarding address.
echo Forwarding to "$ADDRESS"
for h in examples/my*.rs
do
  EXAMPLE=$(basename "$h" .rs)
  perl -pe 's/^(.*?")Paste.*?forward.*? here./${1}$ENV{"ADDRESS"}/' < "$h" > "examples/fwd-$EXAMPLE.rs"
done

cargo run --example fwd-my-11-connecting-devices-using-hub-node-initiator
kill $RESPONDER

# Step 12
cargo run --example my-12-secure-channel-over-hub-node-responder &>responder-12 &
RESPONDER=$!
echo "Waiting 5 seconds for forwarding address.."
sleep 5

# Rewrite "Paste" information with the forwarding address of secure channel
export ADDRESS=$(perl -ne 'm/^Forwarding.*?: (\S+)$/ and print "$1\n"' responder-12)
perl -pe 's/^(.*?")Paste.*?forward.*? here./${1}$ENV{"ADDRESS"}/' < examples/my-12-secure-channel-over-hub-node-initiator.rs > "examples/fwd-my-12-secure-channel-over-hub-node-initiator.rs"

cargo run --example fwd-my-12-secure-channel-over-hub-node-initiator

kill $RESPONDER

# Done
popd >/dev/null || exit 1
popd >/dev/null || exit 1

