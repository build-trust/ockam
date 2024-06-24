ARG OCKAM_VERSION=latest

FROM ghcr.io/build-trust/ockam:${OCKAM_VERSION} as builder
FROM alpine:3

# Install Kafka client
ENV KAFKA_VERSION 3.7.0
ENV SCALA_VERSION 2.13

RUN apk add --update --no-cache curl
RUN apk add --no-cache openjdk17-jre bash bind-tools grep
RUN apk add --no-cache -t .build-deps curl ca-certificates jq
RUN mkdir -p /opt
RUN mirror=$(curl --stderr /dev/null https://www.apache.org/dyn/closer.cgi\?as_json\=1 | jq -r '.preferred') \
  && curl -sSL "${mirror}kafka/${KAFKA_VERSION}/kafka_${SCALA_VERSION}-${KAFKA_VERSION}.tgz" \
  | tar -xzf - -C /opt \
  && mv /opt/kafka_${SCALA_VERSION}-${KAFKA_VERSION} /opt/kafka \
  && adduser -DH -s /sbin/nologin kafka \
  && chown -R kafka: /opt/kafka \
  && rm -rf /tmp/* \
  && apk del --purge .build-deps
ENV PATH "/sbin:/opt/kafka/bin/:$PATH"

# Install Ockam
COPY --from=builder /ockam /usr/local/bin/ockam

# Copy the script that will be used as entrypoint
COPY run_ockam.sh /run_ockam.sh
RUN chmod +x /run_ockam.sh
ENTRYPOINT ["/run_ockam.sh"]
