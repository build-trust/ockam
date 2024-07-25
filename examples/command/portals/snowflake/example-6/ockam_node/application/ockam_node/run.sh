#!/bin/bash
set -ex

# Don't check for the latest Ockam version
export OCKAM_DISABLE_UPGRADE_CHECK=true

# Don't export traces and log messages
export OCKAM_OPENTELEMETRY_EXPORT=false

# print the environment to double-check it
echo "environment variables"
env

ockam node create -vv --foreground --configuration "$CONFIGURATION"
