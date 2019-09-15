#!/bin/sh -eux

cd /tmp
download /opt/node "https://nodejs.org/dist/v10.16.0/node-v10.16.0-linux-x64.tar.gz" \
  "2e2cddf805112bd0b5769290bf2d1bc4bdd55ee44327e826fa94c459835a9d9a"

chown -R root:root /opt/node

echo 'export PATH="/opt/node/bin:$PATH"' > /etc/profile.d/node.sh
. /etc/profile.d/node.sh
chmod u+x /etc/profile.d/node.sh

download /opt/yarn "https://yarnpkg.com/downloads/1.16.0/yarn-v1.16.0.tar.gz" \
  "df202627d9a70cf09ef2fb11cb298cb619db1b958590959d6f6e571b50656029"

mv /tmp/files/yarn-global-packages /opt/
cd /opt/yarn-global-packages
/opt/yarn/bin/yarn install --production --frozen-lockfile
cd /tmp

echo 'export PATH="/opt/yarn-global-packages/node_modules/.bin:/opt/yarn/bin:$PATH"' > /etc/profile.d/yarn.sh
. /etc/profile.d/yarn.sh
chmod u+x /etc/profile.d/yarn.sh
