FROM ghcr.io/build-trust/ockam-python-dev:latest AS dev
COPY requirements.txt ./
RUN pip install -r requirements.txt

FROM ghcr.io/build-trust/ockam-python:latest
COPY --from=dev /app/venv venv
COPY main.py ./
COPY index.html ./
ENTRYPOINT ["python", "main.py"]