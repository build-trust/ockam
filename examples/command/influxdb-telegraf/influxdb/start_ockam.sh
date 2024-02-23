#!/bin/sh

NODE="influxdb"
ockam identity create influxdb
ockam project enroll --identity influxdb --token $OCKAM_TOKEN --project-path /config/project.json
ockam node create $NODE --project-path /config/project.json --identity influxdb
ockam policy create --at $NODE --resource tcp-outlet --expression '(= subject.component "telegraf")'
ockam tcp-outlet create --at /node/$NODE --to 127.0.0.1:8086
ockam relay create $NODE --at /project/$OCKAM_PROJECT_NAME --to /node/$NODE
