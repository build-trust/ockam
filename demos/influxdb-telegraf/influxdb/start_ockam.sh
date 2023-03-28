#!/bin/sh

NODE="influxdb"
ockam identity create influxdb
ockam project authenticate --identity influxdb --token $OCKAM_TOKEN --project-path /config/project.json
ockam node create $NODE --project /config/project.json --identity influxdb
ockam policy create --at $NODE --resource tcp-outlet --expression '(= subject.component "telegraf")'
ockam tcp-outlet create --at /node/$NODE --from /service/outlet --to 127.0.0.1:8086
ockam forwarder create $NODE --at /project/$OCKAM_PROJECT_NAME --to /node/$NODE
