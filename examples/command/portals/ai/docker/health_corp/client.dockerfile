FROM cgr.dev/chainguard/node
ENV NODE_ENV=production

WORKDIR /client

RUN npm install pg
COPY --chown=node:node client.js client.js
ENTRYPOINT [ "node", "client.js" ]
