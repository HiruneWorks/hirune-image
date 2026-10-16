FROM node:24-bookworm-slim
WORKDIR /app
COPY benchmarks/sharp/package*.json ./
RUN npm ci --omit=dev
COPY benchmarks/sharp/*.mjs ./
RUN npm test
USER node
ENV PORT=8080 SHARP_CONCURRENCY=1 REQUEST_CONCURRENCY=1
EXPOSE 8080
ENTRYPOINT ["node", "server.mjs"]
