# Capítulo 11 — Arquitetura spec-driven: fronteiras Node × Rust

> **Princípio:** a spec define quem faz o quê. Se a fronteira entre linguagens não está desenhada, o agente desenha errado — e você paga a conta em runtime.

## Por que separar — a regra de uma frase

Um projeto vibe-coded que mistura tudo num só runtime confunde qualquer agente: ele não sabe se deve usar `fetch` ou `reqwest`, `fs` ou `std::fs`, `npm` ou `cargo`. Separar por responsabilidade — e tornar a separação **visível na estrutura de diretórios** — é o que permite que uma IA (e um humano) escolha a ferramenta certa sem adivinhar.

A regra usada neste repositório, e que o AssinaFácil (cap. 08) copia:

- **Rust faz estado, processos e confiança.** HTTP/WebSocket (`axum` 0.8), banco (`SQLx` + SQLite), git/filesystem/worktrees, spawn de agentes, orquestração. Tudo que precisa ser durável, concorrente ou seguro por tipos.
- **TypeScript faz apresentação e interação.** Componentes React, roteamento (`TanStack Router`), estado de UI, tema, i18n. Tudo que o usuário vê e toca.

A fronteira não é gosto — é contrato. E contratos gerados (cap. 12) são melhores que convenções.

## O mapa do território (caso real — leia com `ls`)

O workspace Cargo (`Cargo.toml` raiz, `edition = "2024"`, `version = "0.2.36"` compartilhada por todos os crates) declara **19 crates**. Não decore — leia em grupos:

| Grupo | Crates | Responsabilidade em uma frase |
| --- | --- | --- |
| **Núcleo** | `server` | Binário principal (axum 0.8, rustls aws-lc-rs): monta o `Router`, serve a API e os assets do frontend; expõe o bin `generate_types` |
|  | `db` | Modelos SQLx + **93 migrations**: `projects`, `issues`, `workspaces`, `sessions`, `execution_processes`, `tags`, `merges`… |
|  | `api-types` | Tipos compartilhados de API consumidos por `server` e `db` (fonte do contrato TS) |
|  | `services` | Lógica de domínio: `local_kanban`, `review_request` (alarme), `pipeline_stage`, `pr_monitor`, `orchestrator_compactor`… |
| **Git & workspaces** | `git` / `git-host` / `worktree-manager` / `workspace-manager` | Operações git, worktrees e ciclo de vida de workspaces (cap. 07) |
| **Execução** | `executors` | Adaptadores dos **11 agentes** (`claude`, `codex`, `gemini`, `opencode`, `cursor`, `amp`, `copilot`, `droid`, `qwen`, `antigravity`, `acp`) + `qa_mock.rs` para testes sem tokens |
| **Integração** | `mcp` | Servidor MCP `vibe-kanban-mcp` e suas tools (`get_pipeline`, `get_rules`, `report_pipeline_stage`…) |
| **Supervisão** | `tui` (`vibe-tui`) / `telegram-bridge` (`vibe-telegram-bridge`) | Cockpit de terminal + daemon send-only que escala approvals para o Telegram |
| **Infra** | `utils`, `client-info`, `server-info`, `local-deployment`, `deployment`, `preview-proxy`, `review`, `tauri-app` | Utilitários, `MsgStore`, detecção de cliente, proxy de preview, review de PRs, app Tauri |

O frontend fica em `packages/`:

| Package | Papel | Onde olhar |
| --- | --- | --- |
| `local-web` | Entrypoint web local (Vite, `app/` + `routes/`, `routeTree.gen.ts` do TanStack Router) | `packages/local-web/src/` |
| `web-core` | Biblioteca compartilhada (`app/`, `features/`, `pages/`, `integrations/`, `i18n/`, `shared/`) | `packages/web-core/src/` |
| `ui` | Design system (consumido por `web-core` e `local-web`) | `packages/ui/` |

O contrato entre os dois mundos fica em `shared/` — assunto do próximo capítulo.

> **Como usar este mapa no seu SaaS:** se o seu AssinaFácil for Node-only, o equivalente é separar `apps/web` (Next), `packages/api` (rotas + DB) e `packages/shared` (tipos). A lição não é "use Rust" — é "cada pasta tem um dono de uma frase, e o dono está escrito no `AGENTS.md`".

## Como a fronteira se materializa — 3 cortes concretos

### 1. API REST + WebSocket (quem fala com quem)

O `server` expõe rotas em `crates/server/src/routes/` — `kanban.rs`, `local_kanban.rs`, `execution_processes.rs`, `workspaces/`, `sessions/`, `approvals.rs`, `events.rs` (stream de logs), `terminal.rs`, `attachments.rs` etc. O frontend em `web-core` consome por `fetch` e `WebSocket` — **nunca acessa o SQLite diretamente**. Se um agente tentar `import { db } from "db"` no frontend, o `check` quebra — e é para quebrar.

### 2. Preview do dev server (quem hospeda quem)

O crate `preview-proxy` (Rust) é o **proxy** que o `server` usa para embutir o dev server do projeto do usuário (Vite/Next em Node) dentro do painel de Preview da workspace (cap. 04). A app do usuário roda em Node; o proxy que a hospeda roda em Rust. Essa é a fronteira encarnada: dois runtimes, um `iframe`.

### 3. Geração de tipos (quem é a fonte)

Structs Rust em `db` e `api-types` com `#[derive(TS)]` viram `shared/types.ts` (cap. 12). O tipo nasce no Rust, atravessa a fronteira **gerado**, e o TypeScript o consome sem redeclaração manual. Um campo novo em `crates/db/src/models/workspace.rs` vira campo em `shared/types.ts` com `pnpm run generate-types` — sem copy-paste.

### Fluxo de um card até virar código em produção

```
UI (React, :3001) ──REST/WS──► server (axum, :3002) ──► services ─┬─► db (SQLite/SQLx, 93 migrations)
                                                                 ├─► worktree-manager (git worktree em /tmp/vibe-kanban/…)
                                                                 ├─► executors (spawn Claude/Codex/… em tmux)
                                                                 └─► MsgStore (log que volta via WS para UI + trackers)
```

O log que os agentes escrevem no `MsgStore` é lido de volta pelo frontend **e** pelos trackers de pipeline (`pipeline_stage.rs`, `review_request.rs`) — o log é, ao mesmo tempo, interface humana e interface de máquina (cap. 13).

## Onde guardar decisões — ADR e o que não deve voltar

Fronteiras geram decisões arquiteturais que precisam sobreviver ao tempo. O repositório tem `docs/ADR/` e a instrução no `AGENTS.md` raiz é explícita:

> "quando uma decisão não-trivial é tomada (novo subsistema, refactor, feature removida), registre como ADR antes ou logo depois, com Status/Date/Context/Decision/Consequences."

O livro que você está lendo é, em parte, um ADR narrado — cada cap. é uma decisão com contexto.

Também existe a seção **"Legacy cloud/remote code"** no `AGENTS.md`: os crates `remote`, `relay-*` foram deletados, e o arquivo lista o que não deve voltar — mas preserva `shared/remote-types.ts` como **contrato congelado** (ver cap. 12). Uma fronteira bem documentada explica não só o que existe, mas **o que foi removido e por quê**. Sem esse registro, alguém apagaria o arquivo achando que é lixo — ou reintroduziria um crate morto.

## Checklist do capítulo

- [ ] Cada diretório de primeiro nível tem uma responsabilidade de uma frase (legível por um agente em 10s).
- [ ] A separação de runtimes está visível na estrutura de pastas (`crates/` vs `packages/` vs `shared/`).
- [ ] Nenhum lado acessa o estado do outro por atalho (frontend não toca SQLite; backend não renderiza JSX).
- [ ] O proxy/preview (se houver) deixa claro quem hospeda quem.
- [ ] Decisões de fronteira estão registradas em ADR, não só em memória ou commit message.
- [ ] O que foi removido tem registro do porquê **e** do que preservar (contratos congelados).
- [ ] Um agente novo, ao ler `AGENTS.md` + `Cargo.toml` + `packages/`, sabe onde criar cada arquivo sem perguntar.
