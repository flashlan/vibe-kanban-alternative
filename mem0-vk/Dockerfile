FROM node:20-slim

WORKDIR /app

COPY package.json package-lock.json ./

RUN npm ci --omit=dev

COPY dist/ ./dist/

# MCP (Streamable-HTTP) + REST endpoints
EXPOSE 8000

CMD ["node", "dist/index.js"]
