#!/bin/sh -eux

download otpsrc \
  "https://github.com/erlang/otp/archive/OTP-22.0.7.tar.gz" \
  "04c090b55ec4a01778e7e1a5b7fdf54012548ca72737965b7aa8c4d7878c92bc"

cd otpsrc
top="$(pwd)"
export ERL_TOP="$top"
./otp_build autoconf
./configure --build="$(dpkg-architecture --query DEB_BUILD_GNU_TYPE)"
make "-j$(nproc)"
make install
cd /tmp
rm -rf otpsrc
