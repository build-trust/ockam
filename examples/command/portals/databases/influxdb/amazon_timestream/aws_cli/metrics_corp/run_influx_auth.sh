#!/bin/bash
set -ex

if [ $# -ne 1 ]; then
  echo "Usage: $0 <host>"
  exit 1
fi

host=$1
password="YourSecurePassword"
org_name="ockam_org"
config_name="test-config"

# Check and delete existing config if it exists
influx config rm "$config_name"

# Create config file
influx config create --active \
  --config-name "$config_name" \
  --host-url "https://$host:8086" \
  --org "$org_name" \
  --username-password "admin:$password"

# Create all access token
token=$(influx auth create --all-access --org "$org_name" --host "https://$host:8086" --json | jq -r .token)

# Get OrgID
org_id=$(influx org list --name "$org_name" --host "https://$host:8086" --json | jq -r '.[0].id')

# Update app.mjs with token and org_id
inputFile="../datastream_corp/app.mjs"
appFile="../datastream_corp/run_app.mjs"
tmpFile="${appFile}.tmp"

# Perform the replacements and write to a temporary file
sed "s/\$TOKEN/${token}/g" "$inputFile" > "$tmpFile"
mv "$tmpFile" "$appFile"
sed "s/\$ORG_ID/${org_id}/g" "$appFile" > "$tmpFile"
mv "$tmpFile" "$appFile"

# Cleanup
influx config rm "$config_name"
