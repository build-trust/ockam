#!/bin/bash
set -ex

echo "Starting Ockam installation..."
if ! curl --proto '=https' --tlsv1.2 -sSfL https://install.command.ockam.io | bash; then
    echo "ERROR: Failed to install Ockam Command"
    exit 1
fi
source "$HOME/.ockam/env"

# Verify Ockam installation
if ! ockam --version; then
    echo "ERROR: Ockam installation verification failed"
    exit 1
else
    echo "Ockam installation verification successful"
fi

echo "Setup ockam node"
export OCKAM_LOG_MAX_FILES=5
export OCKAM_HOME=/opt/ockam_home
export OCKAM_OPENTELEMETRY_EXPORT=false

cat << OCKAMCONFIG > /opt/ockam.json
{
    "tcp-inlet": {
        "from": "0.0.0.0:443",
        "via": "openai-relay",
        "allow": "azure-openai-outlet",
        "tls": true
    }
}
OCKAMCONFIG

echo "Config file: $(cat /opt/ockam.json)"

MARKER_FILE="/opt/ockam_setup_complete.marker"

ENROLLMENT_TICKET=${TICKET}

echo "Check if the $MARKER_FILE file exists to decide to restore or setup a new node"
if [ -f "$MARKER_FILE" ]; then
    echo "Marker file $MARKER_FILE exists, Restoring state"
    ockam node delete --all --yes
    ockam node create /opt/ockam.json
else
    echo "Enrolling project with ticket"
    ockam node create /opt/ockam.json --enrollment-ticket "$ENROLLMENT_TICKET"
    touch "$MARKER_FILE"
fi

echo "Startup script completed successfully"
