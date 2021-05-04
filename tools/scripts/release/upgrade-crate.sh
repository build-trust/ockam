#!/usr/bin/env bash

source "$(dirname "$0")/common.sh"

TARGET="$1"
CRATE="$2"
VERSION="$3"

# ockam = { version = "0.0.0" }
VERS_LINE_SUB="m/^$CRATE\s*=\s*{/ and s/version\s*=\s*\"[^\"]+\"/version = \"${VERSION}\" /"
perl -pe "$VERS_LINE_SUB" < "$TARGET" >"$TARGET.tmp"
mv "$TARGET.tmp" "$TARGET"

# ockam = "0.0.0"
SIMPLE_LINE_SUB="s/^$CRATE\s*=\s*\"[\d\w.-]+\"/$CRATE = \"${VERSION}\"/"
perl -pe "$SIMPLE_LINE_SUB" < "$TARGET" >"$TARGET.tmp"
mv "$TARGET.tmp" "$TARGET"

