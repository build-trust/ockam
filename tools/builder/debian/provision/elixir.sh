#!/bin/sh -eux

download elixirsrc "https://github.com/elixir-lang/elixir/archive/v1.9.0.tar.gz" \
  "dbf4cb66634e22d60fe4aa162946c992257f700c7db123212e7e29d1c0b0c487"

cd elixirsrc
make install clean
cd /tmp
rm -rf elixirsrc
