#!/usr/bin/env bash

FILE=$1
TMP=$(mktemp)
example_blocks "$FILE" >"$TMP"
if ! cmp -s "$FILE" "$TMP"; then
  echo "$FILE examples are not up to date. See diff below."
  echo "===================="
  diff -c "$FILE" "$TMP"
  echo "===================="
  echo
  exit 1
fi
