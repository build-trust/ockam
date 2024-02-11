# This dockerfile builds an image that contains nodejs.
#
# It also copies a bash script called run_ockam.sh from its build directory
# into the image being built and uses that script as entrypoint to containers
# that are run using this image.
#
# The run_ockam.sh script is used to setup an ockam node.

FROM cgr.dev/chainguard/node
ENV NODE_ENV=production

WORKDIR /app

RUN npm install pg
COPY --chown=node:node app.js app.js
ENTRYPOINT [ "node", "app.js" ]
