#!/bin/sh -eux

cd /tmp
download rustup-init "https://static.rust-lang.org/rustup/archive/1.18.3/x86_64-unknown-linux-gnu/rustup-init" \
  "a46fe67199b7bcbbde2dcbc23ae08db6f29883e260e23899a88b9073effc9076"

export CARGO_HOME=/opt/rust/cargo
export PATH=$CARGO_HOME/bin:$PATH
chmod +x rustup-init
./rustup-init -y --no-modify-path --default-toolchain "1.35.0"
rm rustup-init

echo 'export CARGO_HOME=/vagrant/.builder/cargo' > /etc/profile.d/rust.sh
echo 'export PATH="$CARGO_HOME/bin:/opt/rust/cargo/bin:$PATH"' >> /etc/profile.d/rust.sh
chmod u+x /etc/profile.d/rust.sh
