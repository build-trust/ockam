#!/bin/sh

sleep 15

export NAME="telegraf"
# ockam identity create "$NAME"_consumer_identity

# ockam project enroll $CONSUMER_TOKEN --identity "$NAME"_consumer_identity
# ockam node create "$NAME"_consumer_node --identity "$NAME"_consumer_identity
# ockam kafka-consumer create --at "$NAME"_consumer_node

ockam identity create "$NAME"_identity
ockam project enroll $OCKAM_TOKEN --identity "$NAME"_identity
ockam node create "$NAME"_node --identity "$NAME"_identity
ockam kafka-consumer create --at "$NAME"_node

echo "Creating policy..."
ockam policy create --at "$NAME"_node --resource tcp-inlet --expression '(= subject.component "influxdb")'
echo "Creating inlet..."
ockam tcp-inlet create --at /node/"$NAME"_node --from 127.0.0.1:8086 --to /project/default/service/forward_to_influxdb/secure/api/service/outlet
