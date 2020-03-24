#!/bin/sh -eux

file="OpenJDK8U-jdk_x64_linux_hotspot_8u242b08.tar.gz"
download /opt/java/openjdk/jdk8u242-b08 \
  "https://github.com/AdoptOpenJDK/openjdk8-binaries/releases/download/jdk8u242-b08/$file" \
  "f39b523c724d0e0047d238eb2bb17a9565a60574cf651206c867ee5fc000ab43"

file="OpenJDK11U-jdk_x64_linux_hotspot_11.0.6_10.tar.gz"
download /opt/java/openjdk/jdk-11.0.6_10 \
  "https://github.com/AdoptOpenJDK/openjdk11-binaries/releases/download/jdk-11.0.6%2B10/$file" \
  "330d19a2eaa07ed02757d7a785a77bab49f5ee710ea03b4ee2fa220ddd0feffc"

echo 'export JAVA_HOME=/opt/java/openjdk/jdk-11.0.6_10' > /etc/profile.d/java.sh
echo 'export PATH="/opt/java/openjdk/jdk-11.0.6_10/bin:$PATH"' >> /etc/profile.d/java.sh
