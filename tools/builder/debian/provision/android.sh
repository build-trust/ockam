#!/bin/sh -eux

mv /tmp/files/android-sdk /opt/android-sdk

download /opt/android-sdk/platform-tools \
  "https://dl.google.com/android/repository/platform-tools_r29.0.2-linux.zip" \
  "633b6dfa245f5dc58d15da9ead655bcd14de5784196ec0f0dc7e37a5acb61be4"

download /opt/android-sdk/tools \
  "https://dl.google.com/android/repository/sdk-tools-linux-4333796.zip" \
  "92ffee5a1d98d856634e8b71132e8a95d96c83a63fde1099be3d86df3106def9"

download /opt/android-sdk/build-tools/29.0.2 \
  "https://dl.google.com/android/repository/build-tools_r29.0.2-linux.zip" \
  "1e9393cbfd4a4b82e30e7f55ab38db4a5a3259db93d5821c63597bc74522fa08"

download /opt/android-sdk/platforms/android-29 \
  "https://dl.google.com/android/repository/platform-29_r01.zip" \
  "2c9e961858e03900ffa0801a4f6234aa7223363b629fd8e1be60da9aa09d86bd"

download /opt/android-sdk/platforms/android-28 \
  "https://dl.google.com/android/repository/platform-28_r06.zip" \
  "8452dbbf9668a428abb243c4f02a943b7aa83af3cca627629a15c4c09f28e7bd"

download /opt/android-sdk/ndk/20.0.5594570 \
  "https://dl.google.com/android/repository/android-ndk-r20-linux-x86_64.zip" \
  "57435158f109162f41f2f43d5563d2164e4d5d0364783a9a6fab3ef12cb06ce0"


echo 'export ANDROID_HOME="/opt/android-sdk"' > /etc/profile.d/android.sh
echo 'export PATH="/opt/android-sdk/platform-tools:/opt/android-sdk/tools:/opt/android-sdk/tools/bin:$PATH"' \
  >> /etc/profile.d/android.sh

chown -R vagrant:vagrant /opt/android-sdk/*
mkdir -p /home/vagrant/.android
