# Use the same base image as your main application
FROM python:3.10-slim-buster

# Install necessary dependencies
RUN apt-get update && apt-get install -y \
    wget \
    openjdk-11-jre-headless \
    && rm -rf /var/lib/apt/lists/*

# Set up environment variables
ENV KAFKA_VERSION=3.4.1
ENV SCALA_VERSION=2.13
ENV PATH="${PATH}:/opt/kafka/bin"

# Download and extract Kafka
RUN wget -q "https://archive.apache.org/dist/kafka/${KAFKA_VERSION}/kafka_${SCALA_VERSION}-${KAFKA_VERSION}.tgz" -O /tmp/kafka.tgz \
    && tar -xzf /tmp/kafka.tgz -C /opt \
    && rm /tmp/kafka.tgz \
    && ln -s /opt/kafka_${SCALA_VERSION}-${KAFKA_VERSION} /opt/kafka

# Create a script for Kafka operations
RUN echo '#!/bin/bash\n\
if [ -z "$KAFKA_TOPIC" ] || [ -z "$KAFKA_BOOTSTRAP_SERVERS" ]; then\n\
    echo "Error: KAFKA_TOPIC and KAFKA_BOOTSTRAP_SERVERS must be set"\n\
    exit 1\n\
fi\n\
\n\
echo "Kafka Bootstrap Servers: $KAFKA_BOOTSTRAP_SERVERS"\n\
echo "Kafka Topic: $KAFKA_TOPIC"\n\
\n\
echo "Listing available topics:"\n\
kafka-topics.sh --list --bootstrap-server $KAFKA_BOOTSTRAP_SERVERS\n\
\n\
echo "Describing topic $KAFKA_TOPIC:"\n\
kafka-topics.sh --describe --topic $KAFKA_TOPIC --bootstrap-server $KAFKA_BOOTSTRAP_SERVERS\n\
\n\
echo "Checking offsets for $KAFKA_TOPIC:"\n\
kafka-run-class.sh kafka.tools.GetOffsetShell --broker-list $KAFKA_BOOTSTRAP_SERVERS --topic $KAFKA_TOPIC --time -1\n\
\n\
echo "Describing consumer group:"\n\
kafka-consumer-groups.sh --bootstrap-server $KAFKA_BOOTSTRAP_SERVERS --describe --group kafka_ingest_group\n\
\n\
echo "Producing a test message:"\n\
echo "Test message from Docker container" | kafka-console-producer.sh --broker-list $KAFKA_BOOTSTRAP_SERVERS --topic $KAFKA_TOPIC\n\
\n\
echo "Starting Kafka consumer with new group:"\n\
kafka-console-consumer.sh --bootstrap-server $KAFKA_BOOTSTRAP_SERVERS --topic $KAFKA_TOPIC --from-beginning --group new-test-group &\n\
\n\
echo "Starting Kafka consumer without group:"\n\
kafka-console-consumer.sh --bootstrap-server $KAFKA_BOOTSTRAP_SERVERS --topic $KAFKA_TOPIC --from-beginning &\n\
\n\
echo "Checking for ACLs:"\n\
kafka-acls.sh --bootstrap-server $KAFKA_BOOTSTRAP_SERVERS --list\n\
\n\
echo "Starting original Kafka consumer for topic: $KAFKA_TOPIC"\n\
kafka-console-consumer.sh --bootstrap-server $KAFKA_BOOTSTRAP_SERVERS --topic $KAFKA_TOPIC --from-beginning\n\
' > /kafka-ops.sh \
    && chmod +x /kafka-ops.sh

# Set the entrypoint to the Kafka operations script
ENTRYPOINT ["/kafka-ops.sh"]
