#!/bin/sh -eux

cd /tmp

download otpsrc \
  "https://github.com/erlang/otp/archive/OTP-22.2.3.tar.gz" \
  "8470fff519d9ffa5defba4e42c3c1e64cd86905313040246d4a6e35799a9e614"

cd otpsrc
export ERL_TOP="$(pwd)"
./otp_build autoconf
./configure \
    --build="$(dpkg-architecture --query DEB_BUILD_GNU_TYPE)" \
    --without-javac \
    --without-wx \
    --without-debugger \
    --without-observer \
    --without-jinterface \
    --without-cosEvent\
    --without-cosEventDomain \
    --without-cosFileTransfer \
    --without-cosNotification \
    --without-cosProperty \
    --without-cosTime \
    --without-cosTransactions \
    --without-et \
    --without-gs \
    --without-ic \
    --without-megaco \
    --without-orber \
    --without-percept \
    --without-typer \
    --enable-threads \
    --enable-shared-zlib \
    --enable-ssl=dynamic-ssl-lib
make "-j$(nproc)"
make install

cd /tmp
rm -rf otpsrc
