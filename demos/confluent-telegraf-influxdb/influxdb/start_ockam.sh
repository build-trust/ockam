#!/usr/bin/env sh

NAME="influxdb"

ockam identity create $NAME
ockam project enroll $OCKAM_TOKEN --identity $NAME
ockam node create influxdb --identity $NAME
ockam policy create --at $NAME --resource tcp-outlet --expression '(= subject.component "telegraf")'
ockam tcp-outlet create --at "/node/$NAME" --from /service/outlet --to 127.0.0.1:8086
ockam relay create $NAME --at /project/default --to "/node/$NAME"