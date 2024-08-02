ARG BASE_IMAGE=python:3.10-slim-buster
FROM $BASE_IMAGE

# We need both:
#  - The Snowflake library to create a Snowflake connection
#  - The Postgres library to connect to the Postgres host on the Ockam node
RUN pip install --upgrade pip && \
    pip install snowflake && \
    pip install psycopg2-binary

COPY service.py ./
COPY connection.py ./

CMD ["python3", "service.py"]
