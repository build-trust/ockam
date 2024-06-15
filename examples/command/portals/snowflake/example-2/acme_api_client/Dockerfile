ARG BASE_IMAGE=python:3.10-slim
FROM $BASE_IMAGE
COPY client.py ./
RUN pip install --upgrade pip && \
    pip install requests && \
    pip install snowflake && \
    pip install snowflake-connector-python

EXPOSE 15000
CMD ["python", "client.py"]
