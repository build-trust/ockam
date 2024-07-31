#!/bin/bash
set -ex

# Don't check for the latest Ockam version
export OCKAM_DISABLE_UPGRADE_CHECK=true

# Don't export traces and log messages
export OCKAM_OPENTELEMETRY_EXPORT=false

# Print logging
export OCKAM_LOGGING=true

# Don't show colors on the terminal
export NO_COLOR=1

# print the environment to double-check it
echo "show the environment variables"
env

ockam "$@"
