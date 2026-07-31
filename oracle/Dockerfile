# ─────────────────────────────────────────────────────────────────────────────
# StellarLend Oracle – Mock price feed service
#
# Lightweight Node.js service that serves mock asset prices for local
# development. In production this is replaced by a real oracle integration.
# ─────────────────────────────────────────────────────────────────────────────
FROM node:20-alpine
WORKDIR /app
RUN apk add --no-cache wget
COPY package*.json ./
RUN npm ci --omit=dev
COPY . .
EXPOSE 4000
CMD ["node", "server.js"]
