ARG BASE_IMAGE=python:3.10-slim-buster
FROM ${BASE_IMAGE} AS base
# Expose common Kafka-related ports
EXPOSE 9092 9093 2181 29092
FROM ghcr.io/build-trust/ockam AS builder

# Copy the ockam binary from the builder stage to the base image
FROM base AS final
COPY --from=builder  /ockam /usr/local/bin/ockam

# Copy the run.sh script to the base image and set appropriate permissions
COPY --chmod=555 run.sh  /run.sh

# Set the entry point to run.sh
ENTRYPOINT ["/run.sh"]
