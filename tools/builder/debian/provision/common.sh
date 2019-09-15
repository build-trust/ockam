#!/bin/sh -eux

apt-get -y --no-install-recommends install \
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
  gawk
