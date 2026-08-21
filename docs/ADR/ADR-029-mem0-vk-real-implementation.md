# ADR-029: Replace the mem0-vk Stub With the Real Implementation

- **Status**: Accepted
- **Date**: 2026-08-21

## Context

`mem0-vk/` was checked in as a minimal FastAPI (Python) stub (added in `0318a358`, a single commit): `Memory.from_config()` with only `embedder` + `vector_store` (Qdrant), no `llm`/`graph_store` section, and REST handlers that returned placeholder shapes (`/api/usage/tokens` → `{"tokens": 0}`, `/api/config` → `{"config": config}`) that don't match what `crates/server/src/routes/usage.rs` deserializes (`Mem0Config`, `Mem0TokenUsage`).

Meanwhile README.md's "Project Memory (mem0)" and "mem0 Setup" sections already documented the *real* architecture — a Node.js MCP+REST server, a Qdrant vector store, and a Python sentence-transformers + NetworkX graph sidecar (`embeddings`), with runtime-configurable extraction providers and extraction-token monitoring in Settings → Memory/Usage. That real implementation existed only as a separate, independently-developed project (own git history, TypeScript, `src/index.ts`) outside this repository, and was never brought in. Anyone cloning this repo and following the documented `cd mem0-vk && docker compose up -d --build` flow would get a non-functional stub that silently satisfies none of what the docs promise.

The repo also carried a second, redundant `docker-compose.yml` at the repo root (same commit) — a smaller two-service (qdrant + mem0) duplicate of `mem0-vk/docker-compose.yml`, referenced by no script or doc, already drifted from it (missing the `embeddings` service, wrong volume path for the Node server's `/data` mount).

## Decision

- Replaced `mem0-vk/main.py` + `requirements.txt` (Python stub) with the real Node/TypeScript implementation: `src/index.ts` (MCP Streamable-HTTP + REST on the same Qdrant-backed store), `embeddings/` (sentence-transformers + NetworkX graph sidecar), `package.json`/`tsconfig.json`, and the matching `docker-compose.yml`/`Dockerfile`/`.env.example`/`.gitignore`.
- The real server's `/api/config` and `/api/usage/tokens` already match the Rust `Mem0Config`/`Mem0TokenUsage` shapes — verified by running the container and hitting both endpoints.
- Added on-disk persistence for the extraction-token ledger (`recordTokens`/`tokenUsageReport` in `src/index.ts`), mirroring the existing `RUNTIME_CONFIG_PATH` pattern: `MEM0_USAGE_PATH` (default `/data/usage.json`, same mounted `graph_data` volume as the runtime config), loaded on boot, persisted on every token-recording write. Previously the ledger was in-memory only and reset on every container restart.
- Removed the redundant root `docker-compose.yml`. `mem0-vk/docker-compose.yml` (documented in README, self-contained: qdrant + mem0-vk + embeddings) is the single source of truth for the mem0 stack.

## Consequences

- `cd mem0-vk && cp .env.example .env && docker compose up -d --build` (the README-documented flow) now produces a working stack: extraction LLM failover (groq/openrouter/llama/openai), graph entity/relation extraction, and a Settings → Usage "mem0 extraction tokens" panel that populates and survives restarts.
- One compose file for the mem0 stack instead of two drifting definitions.
- Verified end-to-end: a real `POST /api/memories` call ran extraction (groq, `qwen/qwen3.6-27b`), recorded 1754 tokens to the ledger, and the count survived a `docker restart`.
- Trade-off: the Node project's own git history (commits from 2026-08-17 through 2026-08-19) was not preserved — files were copied in at their current state, not merged as history. Acceptable since it was never part of this repo's history to begin with.
