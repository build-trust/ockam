#!/bin/sh -eux

cd /tmp
download cmake_installer "https://github.com/Kitware/CMake/releases/download/v3.14.5/cmake-3.14.5-Linux-x86_64.sh" \
  "1a6082a92be9b11d1e956a2126d0c5ee6d2fbeb70cb419d3b92527e48d6b67c2"

mkdir "/opt/cmake"
chmod u+x cmake_installer
./cmake_installer --skip-license --prefix="/opt/cmake"
rm cmake_installer

echo 'export PATH="/opt/cmake/bin:$PATH"' > /etc/profile.d/cmake.sh
chmod u+x /etc/profile.d/cmake.sh
