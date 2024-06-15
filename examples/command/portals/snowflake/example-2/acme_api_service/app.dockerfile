ARG BASE_IMAGE=python:3.10-slim
FROM $BASE_IMAGE
COPY app.py ./
RUN pip install --upgrade pip && \
    pip install flask

EXPOSE 15000
CMD ["python", "app.py"]
