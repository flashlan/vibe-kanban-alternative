# Appendix — Command reference

Canonical commands for this repository (`package.json` root, `AGENTS.md`). Copy-paste; if one fails, the error teaches the fix (ch. 05).

## Setup

```bash
pnpm i
cp .env.example .env  # if it exists; never commit .env
```

## Development

```bash
pnpm run dev
# Frontend :3001 + Backend :3002 + Preview proxy :3003
# Ports are fixed and exported as FRONTEND_PORT/BACKEND_PORT/PREVIEW_PROXY_PORT

pnpm run backend:dev:watch
# Backend only, with cargo watch (RUST_LOG=debug by default)

pnpm run local-web:dev
# Frontend only (Vite)
```

## Verification (the loop)

```bash
pnpm run check
# local-web:legacy-path-guard + check:db + tsc (local-web, web-core, ui) + cargo check

pnpm run lint
# ESLint (local-web, ui) + cargo clippy -- -D warnings (with --features qa-mode)

pnpm run format
# cargo fmt --all + Prettier (web-core, local-web)
# AGENTS.md mandates it before completing any task.

cargo test --workspace
# Rust tests of all crates
```

## Types and database

```bash
pnpm run generate-types
# Regenerates shared/types.ts from Rust structs (ts-rs)

pnpm run generate-types:check
# Verify only that shared/types.ts is up to date (CI)

pnpm run prepare-db
# Generate .sqlx offline for builds without a database
```

## Automation (ch. 06)

```bash
cargo run -p tui                    # terminal cockpit
cargo run -p telegram-bridge        # Telegram daemon (reads ~/.vibe-kanban/telegram.toml)
cargo run -p mcp -- --mode global   # global MCP server (for the PM agent)
```

## Pipelines and memory (MCP tools)

The MCP tools relevant to a card's flow: `get_rules`, `get_pipeline`, `report_pipeline_stage`, `get_issue`, `update_issue`, `memory_search`, `memory_save`, `respond_to_approval`, `get_orchestrator_prompt`.

Pipelines TOML live in `assets/pipelines/`; `quick.toml` is the trivial-card one.

## Publishing the book

The KDP checklist lives in `docs/livro-vibe-kanban-amazon-checklist.md`. KDP rules: revalidate at `kdp.amazon.com` before publishing — prices and category limits change.
