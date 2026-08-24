# Apêndice — Referência de comandos

Comandos canônicos deste repositório (`package.json` raiz, `AGENTS.md`). Copie e cole; se um falhar, o erro ensina a correção (cap. 5).

## Setup

```bash
pnpm i
cp .env.example .env  # se existir; nunca commite .env
```

## Desenvolvimento

```bash
pnpm run dev
# Frontend :3001 + Backend :3002 + Preview proxy :3003
# As portas são fixas e exportadas como FRONTEND_PORT/BACKEND_PORT/PREVIEW_PROXY_PORT

pnpm run backend:dev:watch
# Só o backend, com cargo watch (RUST_LOG=debug por padrão)

pnpm run local-web:dev
# Só o frontend (Vite)
```

## Verificação (o loop)

```bash
pnpm run check
# local-web:legacy-path-guard + check:db + tsc (local-web, web-core, ui) + cargo check

pnpm run lint
# ESLint (local-web, ui) + cargo clippy -- -D warnings (com --features qa-mode)

pnpm run format
# cargo fmt --all + Prettier (web-core, local-web)
# O AGENTS.md manda rodar antes de completar qualquer task.

cargo test --workspace
# Testes Rust de todos os crates
```

## Tipos e banco

```bash
pnpm run generate-types
# Regenera shared/types.ts a partir dos structs Rust (ts-rs)

pnpm run generate-types:check
# Só verifica se shared/types.ts está atualizado (usado no CI)

pnpm run prepare-db
# Gera .sqlx offline para builds sem banco
```

## Automação (cap. 6)

```bash
cargo run -p tui                    # cockpit de terminal
cargo run -p telegram-bridge        # daemon Telegram (lê ~/.vibe-kanban/telegram.toml)
cargo run -p mcp -- --mode global   # servidor MCP global (para o PM agent)
```

## Pipelines e memória (MCP tools)

As tools MCP relevantes para o fluxo de um card: `get_rules`, `get_pipeline`, `report_pipeline_stage`, `get_issue`, `update_issue`, `memory_search`, `memory_save`, `respond_to_approval`, `get_orchestrator_prompt`.

As pipelines TOML vivem em `assets/pipelines/`; `quick.toml` é a de cards triviais.

## Publicação do livro

O checklist de KDP vive em `docs/livro-vibe-kanban-amazon-checklist.md`. Verificação de regras do KDP: revalide em `kdp.amazon.com` antes de publicar — preços e limites de categoria mudam.
