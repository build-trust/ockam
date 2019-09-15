#!/bin/sh -eux

file="OpenJDK8U-jdk_x64_linux_hotspot_8u212b04.tar.gz"
download /opt/java/openjdk/jdk8u212-b04 \
  "https://github.com/AdoptOpenJDK/openjdk8-binaries/releases/download/jdk8u212-b04/$file" \
  "09f6ccbfd8a86e52665b56751403fd4e2513ad071e2b3f36bab73f86273d1ead"

file="OpenJDK11U-jdk_x64_linux_hotspot_11.0.4_11.tar.gz"
download /opt/java/openjdk/jdk-11.0.4_11 \
  "https://github.com/AdoptOpenJDK/openjdk11-binaries/releases/download/jdk-11.0.4%2B11/$file" \
  "90c33cf3f2ed0bd773f648815de7347e69cfbb3416ef3bf41616ab1c4aa0f5a8"

echo 'export JAVA_HOME=/opt/java/openjdk/jdk-11.0.4_11' > /etc/profile.d/java.sh
echo 'export PATH="/opt/java/openjdk/jdk-11.0.4_11/bin:$PATH"' >> /etc/profile.d/java.sh
