#!/usr/bin/env bash
set -e

# This script automatically fetches the precompiled binaries
# for the latest version released of ockam and stores them.

# Constants definition
# GitHub repos API URLs
API_OCKAM_RELEASES="https://api.github.com/repos/build-trust/ockam/releases"
# Colors
RED="\033[0;31m"
GREEN="\033[0;32m"
ORANGE="\033[0;33m"
NC="\033[0m"  # No Color

# main

OS_TYPE=$(uname)
OS_ARCH=$(uname -m)

case $OS_TYPE in
  "Darwin")
    # Both Intel and M1/M2 Macs will report "x86_64" with "uname -m"
    # https://apple.stackexchange.com/questions/420452/running-uname-m-gives-x86-64-on-m1-mac-mini
    OS_ARCH=$(arch)
    case $OS_ARCH in
      "i386")
        OCKAM_BINARY_FILE="ockam.x86_64-apple-darwin"
        ;;
      "arm64")
        OCKAM_BINARY_FILE="ockam.aarch64-apple-darwin"
        ;;
      *)
        printf "${RED}ERROR${NC} - Unsupported architecture $OS_ARCH discovered, sorry about that!\n"
        ;;
    esac
    ;;
  "Linux")
    case $OS_ARCH in
      "x86_64"|"aarch64")
        OCKAM_BINARY_FILE="ockam.$OS_ARCH-unknown-linux-gnu"
        ;;
      "armv7l")
        OCKAM_BINARY_FILE="ockam.$OS_ARCH-unknown-linux-gnueabihf"
        ;;
      *)
        printf "${RED}ERROR${NC} - Unsupported architecture $OS_ARCH discovered, sorry about that!\n"
    esac
    ;;
  *)
    printf "${RED}ERROR${NC} - Unsupported OS Type $OS_TYPE discovered, sorry about that!\n"
esac

echo

# Check for "cosign", if it's not available we'll skip the verification of the key
printf "Checking if \"cosign\" is available...\t\t\t"
if [ ! "$(which cosign)" ]; then
  printf "${ORANGE}MISSING${NC}\n"
  echo "INFO: The key signature will not be verified"
  SKIP_KEY_VERIFICATION="true"
else
  printf "${GREEN}FOUND${NC}\n"
fi

echo "Prerequisites check completed!"
echo

# Now fetching latest release of ockam and downloading it
printf "Fetching info on latest version of ockam...\t\t"
OCKAM_LATEST_BINARY_QUOTED=$(curl -s "$API_OCKAM_RELEASES" | grep -i browser_download_url | grep "$OCKAM_BINARY_FILE" | head -1 | awk '{print $NF}')
OCKAM_LATEST_BINARY=$(echo "$OCKAM_LATEST_BINARY_QUOTED" | sed -e 's/^"//' -e 's/"$//')
OCKAM_LATEST_VERSION=$(echo "$OCKAM_LATEST_BINARY" | awk -F'/' '{print $(NF-1)}' | cut -d '_' -f2)  # Instead of returning "ockam_v0.75.0" return "v0.75.0"
if [ -z "$OCKAM_LATEST_VERSION" ]; then
  printf "${RED}NOT FOUND{NC}\n"
  exit 1
else
  printf "${GREEN}$OCKAM_LATEST_VERSION${NC}\n"
fi
printf "Fetching ockam $OCKAM_LATEST_VERSION\t\t\t\t\t"
curl -sLO "$OCKAM_LATEST_BINARY"
if [ ! -f "$OCKAM_BINARY_FILE" ]; then
  printf "${RED}FAILED${NC}\n"
  echo "Automated download failed for ockam $OCKAM_LATEST_VERSION, please try to download ockam manually following our documentation:"
  echo "https://docs.ockam.io/get-started"
  exit 1
else
  printf "${GREEN}DONE${NC}\n"
fi

# Download signature and sha256sum
OCKAM_SUMS="sha256sums.txt"
OCKAM_SUMS_SIG="${OCKAM_SUMS}.sig"
OCKAM_RELEASE_PUBLIC_KEY="cosign.pub"

if [ $SKIP_KEY_VERIFICATION ]; then
  printf "Fetching checksum...\t\t\t\t\t"
else
  printf "Fetching signature and checksum...\t\t\t"
  curl -sLO "https://raw.githubusercontent.com/build-trust/ockam/develop/tools/docker/${OCKAM_RELEASE_PUBLIC_KEY}"
  if  [ ! -f "$OCKAM_RELEASE_PUBLIC_KEY" ]; then
    echo "Automated download failed for ockam verification key, please try to download ockam manually following our documentation:"
    echo "https://docs.ockam.io/get-started"
    exit 1
  else
    mv "$OCKAM_RELEASE_PUBLIC_KEY" ockam.pub
  fi
fi

curl -sLO "https://github.com/build-trust/ockam/releases/download/ockam_${OCKAM_LATEST_VERSION}/${OCKAM_SUMS}"
curl -sLO "https://github.com/build-trust/ockam/releases/download/ockam_${OCKAM_LATEST_VERSION}/${OCKAM_SUMS_SIG}"

if [ ! -f "$OCKAM_SUMS" ] || [ ! -f "$OCKAM_SUMS_SIG" ]; then
  printf "${RED}FAILED${NC}\n"
  echo "Automated download failed for ockam checksum, please try to download ockam manually following our documentation:"
  echo "https://docs.ockam.io/get-started"
  exit 1
else
  printf "${GREEN}DONE${NC}\n"
fi

# Verify signature and checksum
if [ $SKIP_KEY_VERIFICATION ]; then
  printf "Verifying checksum...\t\t\t\t\t"
else
  printf "Verifying signature and checksum...\t\t\t"
  if ! cosign verify-blob --key ockam.pub --signature "$OCKAM_SUMS_SIG" "$OCKAM_SUMS" > /dev/null 2>&1; then  # https://www.shellcheck.net/wiki/SC2181
    printf "${RED}FAILED${NC}\n"
    echo "Validation failed for ockam signature, please try to download ockam manually following our documentation:"
    echo "https://docs.ockam.io/get-started"
    exit 1
  fi
fi

# The below "if" line is equivalent to "cat sha256sums.txt | grep ..." with a later "if [ $? -ne 0 ]; then"
# https://www.shellcheck.net/wiki/SC2002
# https://www.shellcheck.net/wiki/SC2181
if ! < "$OCKAM_SUMS" grep "$OCKAM_BINARY_FILE\$" | sha256sum -c > /dev/null 2>&1; then
  printf "${RED}FAILED${NC}\n"
  echo "Validation failed for ockam checksum, please try to download ockam manually following our documentation:"
  echo "https://docs.ockam.io/get-started"
  exit 1
fi
printf "${GREEN}DONE${NC}\n"

printf "Renaming ockam and granting execution permissions...\t"
mv "$OCKAM_BINARY_FILE" ockam
chmod u+x ockam
printf "${GREEN}DONE${NC}\n"

printf "Cleanup of validation files...\t\t\t\t"
if [ $SKIP_KEY_VERIFICATION ]; then
  rm -f $OCKAM_SUMS $OCKAM_SUMS_SIG
else
  rm -f $OCKAM_SUMS $OCKAM_SUMS_SIG ockam.pub
fi
printf "${GREEN}DONE${NC}\n"

echo "Installation completed!"
echo
echo "ockam is now ready to be executed in this directory by launching \"./ockam\"."
echo "You might want to include it in your \$PATH with one of the following options:"
echo
echo "1) If you want to install ockam only for your user, execute:"
echo "mkdir -p $HOME/bin && mv ockam $HOME/bin && echo \"export PATH=$HOME/bin:\$PATH\" >> $HOME/.profile"
echo
echo "2) If you want to install ockam for all users, execute:"
echo "sudo mv ockam /usr/local/bin"
echo
echo "After that, you should be able to execute it anywhere by simply typing \"ockam\"."
echo
echo "Enjoy!"
echo

exit 0
