# Capítulo 3 — Arquitetura spec-driven: fronteiras Node × Rust

> **Princípio:** a spec define quem faz o quê. Se a fronteira entre linguagens não está desenhada, o agente a desenha errado.

## Por que separar

Um projeto vibe-coded que mistura tudo num só runtime confunde qualquer agente: ele não sabe se deve usar `fetch` ou `reqwest`, `fs` ou `std::fs`, `npm` ou `cargo`. Separar por responsabilidade — e tornar a separação visível na estrutura de diretórios — é o que permite que uma IA (e um humano) escolha a ferramenta certa sem adivinhar.

A regra usada neste repositório:

- **Rust faz estado, processos e confiança.** HTTP/WebSocket, banco de dados, git, filesystem, spawn de agentes, orquestração. Tudo que precisa ser durável, concorrente ou seguro por tipos.
- **TypeScript faz apresentação e interação.** Componentes React, roteamento, estado de UI, tema, i18n.

A fronteira não é gosto — é contrato. E contratos gerados são melhores que convenções.

## O mapa do território (caso real)

O workspace Cargo (`Cargo.toml` raiz, `edition = "2024"`, `version = "0.2.36"` compartilhada) declara 19 crates:

| Crate | Responsabilidade em uma frase |
| --- | --- |
| `server` | Binário principal (axum 0.8, rustls aws-lc-rs): monta o `Router`, serve a API e os assets do frontend; bin `generate_types` |
| `db` | Modelos SQLx + 93 migrations: `projects`, `issues`, `workspaces`, `sessions`, `execution_processes`, `tags`, `merges`… |
| `api-types` | Tipos compartilhados de API consumidos por `server`, `db` e pelo contrato TS |
| `services` | Lógica de domínio: `local_kanban`, `review_request` (alarme), `pipeline_stage`, `pr_monitor`, `orchestrator_compactor`… |
| `executors` | Adaptadores dos agentes (Claude, Codex, OpenCode, Gemini, Cursor, Amp, Copilot, Droid, Qwen, Antigravity, ACP…) |
| `git` / `git-host` / `worktree-manager` / `workspace-manager` | Operações git, worktrees e ciclo de vida de workspaces |
| `mcp` | Servidor MCP `vibe-kanban-mcp` e suas tools |
| `tui` | Cockpit de terminal (`vibe-tui`, crate `tui`) |
| `telegram-bridge` | Daemon send-only (`vibe-telegram-bridge`) que escala aprovações para o Telegram |
| `utils`, `client-info`, `server-info` | Utilitários, `MsgStore`, detecção de cliente |
| `local-deployment` / `deployment` / `preview-proxy` / `review` / `tauri-app` | Deployment local, proxy de preview, review de PRs, app Tauri |

O frontend fica em `packages/`:

| Package | Papel |
| --- | --- |
| `local-web` | Entrypoint web local (Vite, `app/` + `routes/`, `routeTree.gen.ts` do TanStack Router) |
| `web-core` | Biblioteca compartilhada (`app/`, `features/`, `pages/`, `integrations/`, `i18n/`, `shared/`) |
| `ui` | Componentes de design system (consumido por `web-core` e `local-web`) |

O contrato entre os dois mundos fica em `shared/` — assunto do próximo capítulo.

## Como a fronteira se materializa

Três cortes concretos neste código:

1. **API REST + WebSocket.** O `server` expõe rotas em `crates/server/src/routes/` (`kanban.rs`, `local_kanban.rs`, `execution_processes.rs`, `workspaces/`, `sessions/`, `approvals.rs`, `events.rs` para o stream de logs, `terminal.rs` etc.). O frontend em `web-core` consome por `fetch` e `WebSocket` — nunca acessa o SQLite diretamente.

2. **Preview do dev server.** O crate `preview-proxy` (Rust) é o proxy que o `server` usa para embutir o dev server do projeto do usuário (Vite/Next) dentro do painel de preview da workspace. A app do usuário roda em Node; o proxy que a hospeda roda em Rust.

3. **Geração de tipos.** Structs Rust em `db` e `api-types` com `#[derive(TS)]` viram `shared/types.ts` (cap. 4). O tipo nasce no Rust, atravessa a fronteira gerado, e o TypeScript o consome sem redeclaração manual.

### Fluxo de um card até virar código em produção

```
UI (React) ──REST/WS──► server (axum) ──► services ─┬─► db (SQLite/SQLx)
                                                    ├─► worktree-manager (git worktree)
                                                    ├─► executors (spawn Claude/Codex/…)
                                                    └─► MsgStore (log que volta via WS)
```

O log que os agentes escrevem no `MsgStore` é lido de volta pelo frontend e pelos trackers de pipeline (`pipeline_stage.rs`, `review_request.rs`) — o log é, ao mesmo tempo, interface humana e interface de máquina (cap. 6).

## Onde guardar decisões

Fronteiras geram decisões arquiteturais que precisam sobreviver ao tempo. O repositório tem `docs/ADR/` e a instrução no `AGENTS.md` raiz é explícita: "quando uma decisão não-trivial é tomada (novo subsistema, refactor, feature removida), registre como ADR antes ou logo depois da implementação, com Status/Date/Context/Decision/Consequences". O livro que você está lendo é, em parte, um ADR narrado.

Também existe a seção "Legacy cloud/remote code" no `AGENTS.md`: os crates `remote`, `relay-*` foram deletados, e o arquivo lista o que não deve voltar — mas preserva `shared/remote-types.ts` como contrato congelado. Uma fronteira bem documentada explica não só o que existe, mas o que foi removido e por quê.

## Checklist do capítulo

- [ ] Cada diretório de primeiro nível tem uma responsabilidade de uma frase.
- [ ] A separação Node × Rust está visível na estrutura de pastas (`crates/` vs `packages/` vs `shared/`).
- [ ] Nenhum lado acessa o estado do outro por atalho (frontend não toca SQLite; backend não renderiza JSX).
- [ ] O proxy/preview (se houver) deixa claro quem hospeda quem.
- [ ] Decisões de fronteira estão registradas em ADR, não só em memória.
- [ ] O que foi removido tem registro do porquê e do que preservar (contratos congelados).
