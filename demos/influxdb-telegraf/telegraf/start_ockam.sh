#!/bin/sh
NODE="telegraf"
ockam identity create telegraf
ockam project authenticate --identity telegraf --token $OCKAM_TOKEN --project-path /config/project.json
ockam node create $NODE --project /config/project.json --identity telegraf
ockam policy set --at $NODE --resource tcp-inlet --expression '(= subject.component "influxdb")'
sleep 30
ockam tcp-inlet create --at /node/$NODE --from 127.0.0.1:8086 --to /project/$OCKAM_PROJECT_NAME/service/forward_to_influxdb/secure/api/service/outlet
