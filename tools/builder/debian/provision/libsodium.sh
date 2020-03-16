#!/bin/sh -eux

cd /tmp

download libsodium "https://download.libsodium.org/libsodium/releases/libsodium-1.0.18.tar.gz" \
  "6f504490b342a4f8a4c4a02fc9b866cbef8622d5df4e5452b46be121e46636c1"

cd libsodium
./configure && make && make install

cd /tmp
rm -rf libsodium
