FROM oven/bun:1 AS builder
WORKDIR /app
ARG PUBLIC_API_BASE_URL=http://localhost:4000
ENV PUBLIC_API_BASE_URL=$PUBLIC_API_BASE_URL
COPY package.json bun.lock ./
RUN bun install --frozen-lockfile
COPY . .
RUN bun run build

FROM oven/bun:1-slim
WORKDIR /app
COPY --from=builder /app/build ./build
COPY --from=builder /app/node_modules ./node_modules
COPY --from=builder /app/package.json ./
EXPOSE 3000
ENV HOST=0.0.0.0
CMD ["bun", "build/index.js"]
