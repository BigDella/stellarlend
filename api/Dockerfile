# ─────────────────────────────────────────────────────────────────────────────
# StellarLend API – Multi-stage Dockerfile
#
# Stages:
#   base        – shared Node.js base (deps only)
#   development – hot-reload via ts-node-dev (used by docker compose)
#   builder     – compiles TypeScript to JS
#   production  – minimal runtime image
#
# ARM / M1 compatible: node:20-alpine ships linux/amd64 + linux/arm64 manifests
# ─────────────────────────────────────────────────────────────────────────────

# ── Base ──────────────────────────────────────────────────────────────────────
FROM node:20-alpine AS base
WORKDIR /app
# Install wget for healthcheck
RUN apk add --no-cache wget
COPY package*.json ./

# ── Development (hot-reload) ──────────────────────────────────────────────────
FROM base AS development
# Install ALL deps (including devDependencies for ts-node-dev)
RUN npm ci
COPY . .
EXPOSE 3000
CMD ["npm", "run", "dev"]

# ── Builder ───────────────────────────────────────────────────────────────────
FROM base AS builder
RUN npm ci
COPY . .
RUN npm run build

# ── Production ────────────────────────────────────────────────────────────────
FROM node:20-alpine AS production
WORKDIR /app
RUN apk add --no-cache wget
COPY package*.json ./
RUN npm ci --omit=dev
COPY --from=builder /app/dist ./dist
EXPOSE 3000
CMD ["node", "dist/index.js"]
