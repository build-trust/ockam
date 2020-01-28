#!/bin/sh -eux

cd /tmp

download elixirsrc "https://github.com/elixir-lang/elixir/archive/v1.10.0.tar.gz" \
  "6f0d35acfcbede5ef7dced3e37f016fd122c2779000ca9dcaf92975b220737b7"

cd elixirsrc
make && make install
mix local.hex --force
mix local.rebar --force

cd /tmp
rm -rf elixirsrc
