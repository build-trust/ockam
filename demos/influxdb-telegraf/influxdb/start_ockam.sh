#!/bin/sh

NODE="influxdb"
ockam node create $NODE --project /config/project.json --enrollment-token $OCKAM_TOKEN
ockam policy set --at $NODE --resource tcp-outlet --expression '(= subject.component "telegraf")'
ockam tcp-outlet create --at /node/$NODE --from /service/outlet --to 127.0.0.1:8086
ockam forwarder create $NODE --at /project/$OCKAM_PROJECT_NAME --to /node/$NODE
