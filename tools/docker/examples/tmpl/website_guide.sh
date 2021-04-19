pushd "$OCKAM_HOME/implementations/rust/ockam/ockam_examples" >/dev/null || exit 1

# Build all examples
for e in examples/*.rs
do
  cargo build --example "$(basename "$e" .rs)" || exit 2
done


# Step 1
cargo run --example step-1

# Step 2
cargo run --example step-2-server &
SERVER=$!
sleep 2
cargo run --example step-2-client
kill $SERVER

# Rewrite the "Paste" instructions to the hub instance.
for h in examples/*.rs
do
  EXAMPLE=$(basename "$h" .rs)
  perl -pe 's/^(.*?")Paste.*Hub here./${1}$ENV{"OCKAM_HUB"}/' < "$h" > "examples/my-$EXAMPLE.rs"
done

# Step 3

cargo run --example my-step-3

# Step 4

cargo run --example my-step-4-server &>server-4 &
SERVER=$!
echo "Waiting 7 seconds for forwarding address.."
sleep 7
export ADDRESS=$(perl -ne 'm/[fF]orwarding.*?: (\S+)$/ and print "$1\n"' server-4)

echo Forwarding to "$ADDRESS"
for h in examples/my*.rs
do
  EXAMPLE=$(basename "$h" .rs)
  perl -pe 's/^(.*?")Paste.*?forward.*? here./${1}$ENV{"ADDRESS"}/' < "$h" > "examples/fwd-$EXAMPLE.rs"
done

cargo run --example fwd-my-step-4-client
kill $SERVER

# Step 5

cargo run --example my-step-5-server &>server-5 &
SERVER=$!
echo "Waiting 7 seconds for forwarding address.."
sleep 7
export ADDRESS=$(perl -ne 'm/address: (\S+)$/ and print "$1\n"' server-5)

echo Forwarding to "$ADDRESS"
for h in examples/my*.rs
do
  EXAMPLE=$(basename "$h" .rs)
  perl -pe 's/^(.*?")Paste.*?forward.*? here./${1}$ENV{"ADDRESS"}/' < "$h" > "examples/fwd-$EXAMPLE.rs"
done

cargo run --example fwd-my-step-5-client
kill $SERVER

