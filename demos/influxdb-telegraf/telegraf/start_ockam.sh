#!/bin/sh
NODE="telegraf"
ockam node create $NODE --project /config/project.json --enrollment-token $OCKAM_TOKEN
ockam policy set --at $NODE --resource tcp-inlet --expression '(= subject.component "influxdb")'
sleep 30
ockam tcp-inlet create --at /node/$NODE --from 127.0.0.1:8086 --to /project/$OCKAM_PROJECT_NAME/service/forward_to_influxdb/secure/api/service/outlet
