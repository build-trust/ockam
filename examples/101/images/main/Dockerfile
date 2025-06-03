FROM ghcr.io/build-trust/ockam-python-dev:latest AS dev

ENV PATH="/app/venv/bin:$PATH"
COPY requirements.txt requirements.txt
RUN python -m venv /app/venv && pip install -r requirements.txt

FROM ghcr.io/build-trust/ockam-python:latest

WORKDIR /app

COPY . .
COPY --from=dev /app/venv /app/venv
ENV PATH="/app/venv/bin:$PATH"

ENTRYPOINT ["python", "main.py"]
