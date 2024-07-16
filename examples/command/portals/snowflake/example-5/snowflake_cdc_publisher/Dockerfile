ARG BASE_IMAGE=python:3.10-slim-buster

FROM ghcr.io/build-trust/ockam as ockam

FROM $BASE_IMAGE

RUN pip install --upgrade pip && \
    pip install confluent-kafka && \
    pip install snowflake && \
    pip install snowflake-connector-python

COPY snowflake_cdc_publisher.py ./
COPY --from=ockam /ockam /usr/local/bin/ockam
COPY --chmod=555 run.sh  /run.sh

ENTRYPOINT ["/run.sh"]

