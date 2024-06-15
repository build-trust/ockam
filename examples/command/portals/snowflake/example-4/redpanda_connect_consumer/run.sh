#!/bin/bash
set -e

url_encode() {
  python -c "import urllib.parse; print(urllib.parse.quote('$1', safe=''))"
}

token=$(cat /snowflake/session/token)

encoded_token=$(url_encode "$token")

export SNOWFLAKE_TOKEN="$encoded_token"

SNOWFLAKE_TOKEN="$encoded_token" redpanda-connect --config /consumer.yaml
