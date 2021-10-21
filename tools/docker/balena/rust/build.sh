TARGET_INTEL=x86_64-unknown-linux-musl
TARGET_ARM=aarch64-unknown-linux-musl
BIN=release/balena_ockam

cargo build --release --target $TARGET_INTEL &&
cargo build --release --target $TARGET_ARM

aarch64-linux-gnu-strip target/$TARGET_ARM/$BIN &&
strip target/$TARGET_INTEL/$BIN

cp target/$TARGET_ARM/$BIN ../dist/aarch64
cp target/$TARGET_INTEL/$BIN ../dist/amd64
cargo clean
