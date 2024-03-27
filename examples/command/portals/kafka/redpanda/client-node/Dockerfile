FROM alpine:3

ENV KAFKA_VERSION 3.7.0
ENV SCALA_VERSION 2.13

RUN apk add --update --no-cache curl
RUN apk add --no-cache openjdk17-jre bash
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

RUN curl --proto '=https' --tlsv1.2 -sSf https://raw.githubusercontent.com/build-trust/ockam/develop/install.sh | bash
ENV PATH "/root/.ockam/bin:/sbin:/opt/kafka/bin/:$PATH"
COPY ./consumer.sh /consumer.sh
RUN chmod +x /consumer.sh
COPY ./producer.sh /producer.sh
RUN chmod +x /producer.sh

