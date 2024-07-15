ARG BASE_IMAGE=python:3.10-slim-buster
FROM $BASE_IMAGE
COPY echo_service.py ./
COPY templates/ ./templates/
RUN pip install --upgrade pip && \
    pip install flask
CMD ["python3", "service.py"]
