#!/bin/bash
set -e
#############
# Usage
# DB_ENDPOINT="SQLSERVER_HOST:1433" ./setup_ockam_outlet.sh
#############

# Check if mssql_outlet.ticket exists
if [ ! -f "mssql_outlet.ticket" ]; then
    echo "ERROR: mssql_outlet.ticket file not found"
    exit 1
fi

# Validate DB_ENDPOINT existence and format
if [ -z "$DB_ENDPOINT" ] || ! echo "$DB_ENDPOINT" | grep -qE '^[a-zA-Z0-9.-]+:[0-9]+$'; then
    echo "ERROR: DB_ENDPOINT must be set and in format hostname:port"
    exit 1
fi

echo "Starting Ockam installation..."
if ! curl --proto '=https' --tlsv1.2 -sSfL https://install.command.ockam.io | bash; then
    echo "ERROR: Failed to install Ockam Command"
    exit 1
fi

source "$HOME/.ockam/env"

echo "Setup ockam node"
export OCKAM_LOG_MAX_FILES=5
export OCKAM_HOME=/opt/ockam_home

echo "Enrolling project with ticket"
ockam node create --enrollment-ticket "$(cat mssql_outlet.ticket)" \
--configuration "
{
    "relay": "mssql",
    "tcp-outlet": {
        "to": "${DB_ENDPOINT}",
        "allow": "snowflake"
    }
}
"

echo "Ockam setup complete"
