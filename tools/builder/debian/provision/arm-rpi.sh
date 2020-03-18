#!/bin/sh -eux

cd /tmp

download arm-rpi "https://github.com/raspberrypi/tools/archive/b0c869bc929587a7e1d20a98e2dc828a24ca396a.zip" \
  "a14d46623b3b35743efb81d6c4096088f7b13796887670bb58dfc54446df1d4a"

mkdir -p "/opt/arm-rpi-4.9.3-linux-gnueabihf"
mv arm-rpi/arm-bcm2708/arm-rpi-4.9.3-linux-gnueabihf/* /opt/arm-rpi-4.9.3-linux-gnueabihf

echo 'export PATH="/opt/arm-rpi-4.9.3-linux-gnueabihf/bin:$PATH"' > /etc/profile.d/arm-rpi.sh
chmod u+x /etc/profile.d/arm-rpi.sh

