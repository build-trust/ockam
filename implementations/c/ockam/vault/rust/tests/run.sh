#!/bin/sh

cargo build --features=ffi --release --manifest-path=../../../../../rust/vault/Cargo.toml
gcc -I../../../../../rust/vault/include -L../../../../../rust/target/release -o vault -lockam_vault vault.c
./vault