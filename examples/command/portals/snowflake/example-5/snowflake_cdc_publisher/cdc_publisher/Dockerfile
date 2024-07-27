ARG BASE_IMAGE=python:3.10-slim-buster

FROM $BASE_IMAGE

RUN pip install --upgrade pip && \
    pip install confluent-kafka && \
    pip install snowflake && \
    pip install snowflake-connector-python

COPY snowflake_cdc_publisher.py ./

CMD ["python", "snowflake_cdc_publisher.py"]
