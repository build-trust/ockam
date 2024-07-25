ARG BASE_IMAGE=python:3.10-slim-buster

FROM ghcr.io/build-trust/ockam AS ockam

FROM $BASE_IMAGE

COPY --from=ockam /ockam /usr/local/bin/ockam
COPY --chmod=555 run.sh  /run.sh

ENTRYPOINT ["/run.sh"]
