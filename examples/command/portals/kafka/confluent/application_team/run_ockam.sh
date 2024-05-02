#!/usr/bin/env bash
set -e

cat >kafka.config <<EOF
request.timeout.ms=30000
security.protocol=SASL_PLAINTEXT
sasl.mechanism=PLAIN
sasl.jaas.config=org.apache.kafka.common.security.plain.PlainLoginModule required \
        username="$CONFLUENT_CLOUD_KAFKA_CLUSTER_API_KEY" \
        password="$CONFLUENT_CLOUD_KAFKA_CLUSTER_API_SECRET";
EOF

# Orders the time required to enroll for the consumer and producer
if echo "$@" | grep kafka-console-producer.sh; then
  sleep 17;
else
  sleep 10;
fi;

# Enroll with our enrollment ticket as an authorized user that can access
# the Confluent Kafka relay.
ockam project enroll "$ENROLLMENT_TICKET"

if echo "$@" | grep kafka-console-producer.sh; then
  # create a kafka inlet that'll be used by the Producer
  ockam kafka-producer create --bootstrap-server 127.0.0.1:9092
else
  # create a kafka inlet that'll be used by the Consuner
  ockam kafka-consumer create --bootstrap-server 127.0.0.1:9092
fi;

# Execute the command specified in 'docker-compose.yml'
bash "$@"
