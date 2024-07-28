ARG BASE_IMAGE=python:3.10-slim-buster

FROM $BASE_IMAGE

RUN pip install --upgrade pip && \
    pip install confluent-kafka && \
    pip install snowflake && \
    pip install snowflake-connector-python

COPY connection.py ./
COPY kafka_reader.py ./

CMD ["python", "kafka_reader.py"]
