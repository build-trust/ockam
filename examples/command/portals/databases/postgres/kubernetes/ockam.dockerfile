# This dockerfile builds an image that contains bash and ockam command.
#
# It also copies a bash script called run_ockam.sh from its build directory
# into the image being built and uses that script as entrypoint to containers
# that are run using this image.
#
# The run_ockam.sh script is used to set up and start an ockam node.
#
# Read bank_corp/run_ockam.sh and analysis_corp/run_ockam.sh to understand
# how each node is set up.

ARG OCKAM_VERSION=latest

FROM ghcr.io/build-trust/ockam:${OCKAM_VERSION} as builder

FROM cgr.dev/chainguard/bash
COPY --from=builder /ockam /usr/local/bin/ockam

COPY run_ockam.sh /run_ockam.sh
RUN chmod +x /run_ockam.sh
ENTRYPOINT ["/run_ockam.sh"]
