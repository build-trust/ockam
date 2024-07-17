ARG BASE_IMAGE=python:3.10-slim-buster
FROM $BASE_IMAGE

RUN pip install --upgrade pip && \
    pip install confluent-kafka && \
    pip install snowflake && \
    pip install snowflake-connector-python

COPY service.py ./
COPY connection.py ./

CMD ["python3", "service.py"]
