# Multi-stage: compile TypeScript inside the build (dist/ is gitignored and
# never committed — a fresh clone has no dist/ on disk, so `COPY dist/` alone
# would fail `docker compose up -d --build` for anyone who hadn't happened to
# run `npm run build` locally first). This makes the documented Quick Start
# (`cd mem0-vk && docker compose up -d --build`) actually self-contained.
FROM node:20-slim AS build

WORKDIR /app

COPY package.json package-lock.json ./
RUN npm ci

COPY tsconfig.json ./
COPY src/ ./src/
RUN npm run build

# ── Runtime: only the compiled output + prod deps, no TypeScript/dev tools ──
FROM node:20-slim

WORKDIR /app

COPY package.json package-lock.json ./
RUN npm ci --omit=dev

COPY --from=build /app/dist/ ./dist/

# MCP (Streamable-HTTP) + REST endpoints
EXPOSE 8000

CMD ["node", "dist/index.js"]
