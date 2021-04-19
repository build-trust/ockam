#!/usr/bin/env bash

REV=$1
if [ -z "$REV" ]
then
  REV=develop
fi
export REV


echo "Checking out $REV"
git clone https://github.com/ockam-network/ockam.git
cd ockam
git checkout $REV -q

