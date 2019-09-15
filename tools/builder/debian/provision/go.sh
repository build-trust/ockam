#!/bin/sh -eux

cd /tmp
download /opt/go "https://dl.google.com/go/go1.12.6.linux-amd64.tar.gz" \
  "dbcf71a3c1ea53b8d54ef1b48c85a39a6c9a935d01fc8291ff2b92028e59913c"

echo 'export GOPATH=/vagrant/.builder/go' > /etc/profile.d/go.sh
echo 'export PATH="$GOPATH/bin:/opt/go/bin:$PATH"' >> /etc/profile.d/go.sh
chmod u+x /etc/profile.d/go.sh
