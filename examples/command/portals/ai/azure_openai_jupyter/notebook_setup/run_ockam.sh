#!/bin/bash
set -e

curl --proto '=https' --tlsv1.2 -sSfL https://install.command.ockam.io | bash
source "$HOME/.ockam/env"
export OCKAM_LOG_MAX_FILES=5
export OCKAM_HOME="$HOME/.ockam"
export OCKAM_OPENTELEMETRY_EXPORT=false

cat << OCKAMCONFIG > /home/jovyan/ockam.json
{
    "tcp-inlet": {
        "from": "0.0.0.0:443",
        "via": "openai-relay",
        "allow": "azure-openai-outlet",
        "tls": true
    }
}
OCKAMCONFIG
ockam node create /home/jovyan/ockam.json --enrollment-ticket "$ENROLLMENT_TICKET"
