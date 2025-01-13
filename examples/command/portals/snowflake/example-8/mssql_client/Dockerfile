ARG BASE_IMAGE=python:3.10-slim-buster
FROM $BASE_IMAGE

RUN pip install --upgrade pip && \
    pip install flask snowflake snowflake-connector-python snowflake-snowpark-python pandas

RUN pip install pymssql==2.2.11

COPY service.py ./
COPY connection.py ./

CMD ["python3", "service.py"]
