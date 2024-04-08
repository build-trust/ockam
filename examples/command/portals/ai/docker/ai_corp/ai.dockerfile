FROM cgr.dev/chainguard/node
ENV NODE_ENV=production

WORKDIR /api
COPY --chown=node:node models models
COPY --chown=node:node ai.mjs ai.mjs

RUN npm install express node-llama-cpp
ENTRYPOINT [ "node", "ai.mjs" ]
