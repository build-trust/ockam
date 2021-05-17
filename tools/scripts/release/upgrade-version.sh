#!/usr/bin/env bash

source "$(dirname "$0")/common.sh"

TARGET="$1"
VERSION="$2"

# version = "0.0.0"
LINE_SUB="s/^version\s*=\s*\"[\d\w.-]+\"/version = \"${VERSION}\"/"
perl -pe "$LINE_SUB" < "$TARGET" >"$TARGET.tmp"
mv "$TARGET.tmp" "$TARGET"

