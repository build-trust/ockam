ARG BASE_IMAGE=python:3.12

FROM ghcr.io/build-trust/redpanda-connect as redpanda-connect
FROM ghcr.io/build-trust/ockam as ockam

FROM $BASE_IMAGE
COPY --from=redpanda-connect --chmod=555 /redpanda-connect /usr/local/bin/redpanda-connect
COPY --from=ockam /ockam /usr/local/bin/ockam

WORKDIR /
COPY consumer.yaml /consumer.yaml
COPY --chmod=555 run.sh  /run.sh

ENTRYPOINT ["/run.sh"]
