#!/bin/sh -eux

sudo echo 'deb http://deb.debian.org/debian unstable main' >> /etc/apt/sources.list

sudo apt-get update

apt-get -y --no-install-recommends -t unstable install \
  libc6 \
  libc6-dev \
  openssl \
  libssl-dev

apt-get -y --no-install-recommends -t stretch install \
  dpkg-dev \
  g++ \
  libstdc++-6-dev \
  autoconf \
  automake \
  libncurses5-dev \
  libssl-dev \
  unixodbc-dev \
  libsctp-dev \
  libwxgtk3.0-dev \
  xsltproc \
  fop \
  libxml2-utils \
  unzip \
  gperf \
  bison \
  flex \
  texinfo \
  help2man \
  python3-dev \
  libtool-bin \
  gawk \
  llvm-dev \
  libclang-dev \
  clang \
  git
