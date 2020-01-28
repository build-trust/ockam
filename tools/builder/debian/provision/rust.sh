#!/bin/sh -eux

cd /tmp

mkdir -p /opt/rust/cargo
export CARGO_HOME=/opt/rust/cargo
export RUSTUP_HOME=/opt/rust/rustup
export PATH=$CARGO_HOME/bin:$PATH

curl --proto '=https' --tlsv1.2 -sSf 'https://sh.rustup.rs' > /tmp/rustup-init
chmod +x /tmp/rustup-init

/tmp/rustup-init -y \
    --no-modify-path \
    --profile minimal \
    --default-toolchain stable \
    --component rustfmt \
    --target armv7-unknown-linux-gnueabihf

rm /tmp/rustup-init

echo "export CARGO_HOME=$CARGO_HOME" > /etc/profile.d/rust.sh
echo "export RUSTUP_HOME=$RUSTUP_HOME" > /etc/profile.d/rust.sh
echo "export PATH=\"$CARGO_HOME/bin:\$PATH\"" >> /etc/profile.d/rust.sh
chmod u+x /etc/profile.d/rust.sh
