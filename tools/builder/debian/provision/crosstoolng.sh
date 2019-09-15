#!/bin/sh -eux

cd /tmp
download crosstoolngsrc "http://crosstool-ng.org/download/crosstool-ng/crosstool-ng-1.24.0.tar.xz" \
  "804ced838ea7fe3fac1e82f0061269de940c82b05d0de672e7d424af98f22d2d"

cd crosstoolngsrc
./configure --prefix=/opt/crosstool-ng
make
make install

cd /tmp
rm -rf crosstoolngsrc
echo 'export PATH="/opt/crosstool-ng/bin:$PATH"' > /etc/profile.d/crosstoolng.sh
chmod u+x /etc/profile.d/crosstoolng.sh
