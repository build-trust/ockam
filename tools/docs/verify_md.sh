#!/usr/bin/env bash

FILE=$1
TMP=$(mktemp)
example_blocks $FILE >$TMP
cmp -s $FILE $TMP
if [[ $? -ne 0 ]]; then
  echo "$FILE examples are not up to date."
  exit 1
fi
