ARG BASE_IMAGE=python:3.10-slim-buster
FROM $BASE_IMAGE

RUN pip install --upgrade pip && \
    pip install snowflake && \
    pip install psycopg2-binary

COPY service.py ./
COPY connection.py ./

CMD ["python3", "service.py"]
