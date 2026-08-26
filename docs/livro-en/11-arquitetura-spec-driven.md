# Chapter 11 — Spec-driven architecture: Node × Rust boundaries

> **Principle:** the spec defines who does what. If the boundary between languages isn't drawn, the agent draws it wrong.

## Why separate — the one-sentence rule

A vibe-coded project that mixes everything in one runtime confuses any agent: it doesn't know whether to use `fetch` or `reqwest`, `fs` or `std::fs`, `npm` or `cargo`. Separate by responsibility — and make the separation **visible in the directory structure** — is what lets an AI (and a human) pick the right tool without guessing.

The rule used here:

- **Rust does state, processes and trust.** HTTP/WebSocket, database, git, filesystem, agent spawning, orchestration. Whatever must be durable, concurrent or type-safe.
- **TypeScript does presentation and interaction.** React components, routing, UI state, theme, i18n.

The boundary isn't taste — it's a contract. And generated contracts (ch. 12) beat conventions.

## The territory (real case — read with `ls`)

The Cargo workspace (`Cargo.toml` root, `edition = "2024"`, `version = "0.2.41"` shared) declares **19 crates**. Read them in groups:

| Group | Crates | Responsibility in one sentence |
| --- | --- | --- |
| **Core** | `server` | Main binary (axum 0.8, rustls aws-lc-rs): mounts the `Router`, serves the API and frontend assets; exposes `generate_types` |
|  | `db` | SQLx models + **93 migrations**: `projects`, `issues`, `workspaces`, `sessions`, `execution_processes`, `tags`, `merges`… |
|  | `api-types` | Shared API types consumed by `server` and `db` (source of the TS contract) |
|  | `services` | Domain logic: `local_kanban`, `review_request` (alarm), `pipeline_stage`, `pr_monitor`, `orchestrator_compactor`… |
| **Git & workspaces** | `git` / `git-host` / `worktree-manager` / `workspace-manager` | Git ops, worktrees and workspace lifecycle (ch. 07) |
| **Execution** | `executors` | One module per supported agent (`claude`, `codex`, `gemini`, `opencode`, `cursor`, `amp`, `copilot`, `droid`, `qwen`, `antigravity`, `acp`) + `qa_mock.rs` for tests without tokens |
| **Integration** | `mcp` | The `vibe-kanban-mcp` server and its tools (`get_pipeline`, `get_rules`, `report_pipeline_stage`…) |
| **Supervision** | `tui` (`vibe-tui`) / `telegram-bridge` (`vibe-telegram-bridge`) | Terminal cockpit + send-only daemon escalating approvals to Telegram |
| **Infra** | `utils`, `client-info`, `server-info`, `local-deployment`, `deployment`, `preview-proxy`, `review`, `tauri-app` | Utilities, `MsgStore`, client detection, preview proxy, PR review, Tauri app |

The frontend lives in `packages/`:

| Package | Role | Where to look |
| --- | --- | --- |
| `local-web` | Local web entrypoint (Vite, `app/` + `routes/`, `routeTree.gen.ts` from TanStack Router) | `packages/local-web/src/` |
| `web-core` | Shared library (`app/`, `features/`, `pages/`, `integrations/`, `i18n/`, `shared/`) | `packages/web-core/src/` |
| `ui` | Design system (consumed by `web-core` and `local-web`) | `packages/ui/` |

The contract between the two worlds lives in `shared/` — subject of the next chapter.

> **How to use this map in your SaaS:** if your AssinaFácil is Node-only, the equivalent is separating `apps/web` (Next), `packages/api` (routes + DB) and `packages/shared` (types). The lesson isn't "use Rust" — it's "each folder has a one-sentence owner, and the owner is written in `AGENTS.md`".

## How the boundary materializes — 3 concrete cuts

### 1. API REST + WebSocket (who talks to whom)

`server` exposes routes in `crates/server/src/routes/` — `kanban.rs`, `local_kanban.rs`, `execution_processes.rs`, `workspaces/`, `sessions/`, `approvals.rs`, `events.rs` (log stream), `terminal.rs`, `attachments.rs`… The frontend in `web-core` consumes via `fetch` and `WebSocket` — **never touches SQLite directly**. If an agent tries `import { db } from "db"` in the frontend, `check` breaks — and it should.

### 2. Preview of the dev server (who hosts whom)

The `preview-proxy` crate (Rust) is the **proxy** that `server` uses to embed the user's dev server (Vite/Next in Node) inside the workspace Preview panel (ch. 04). The user's app runs in Node; the proxy hosting it runs in Rust. That's the boundary incarnate: two runtimes, one `iframe`.

### 3. Type generation (who is the source)

Rust structs in `db` and `api-types` with `#[derive(TS)]` become `shared/types.ts` (ch. 12). The type is born in Rust, crosses the boundary **generated**, and TypeScript consumes it without redeclaration.

### Flow of a card to production code

```
UI (React, :3001) ──REST/WS──► server (axum, :3002) ──► services ─┬─► db (SQLite/SQLx, 93 migrations)
                                                                 ├─► worktree-manager (git worktree in /tmp/vibe-kanban/…)
                                                                 ├─► executors (spawn Claude/Codex/… in tmux)
                                                                 └─► MsgStore (log back via WS to UI + trackers)
```

The log the agents write in `MsgStore` is read back by the frontend **and** by the pipeline trackers (`pipeline_stage.rs`, `review_request.rs`) — the log is both human interface and machine interface (ch. 13).

## Where to keep decisions — ADR and what must not return

Boundaries generate architectural decisions that must outlive the moment. The repo has `docs/ADR/` and the root `AGENTS.md` is explicit:

> "when a non-trivial decision is made (new subsystem, refactor, removed feature), record it as an ADR before or right after, with Status/Date/Context/Decision/Consequences."

This book is, in part, a narrated ADR. There's also the **"Legacy cloud/remote code"** section in `AGENTS.md`: crates `remote`, `relay-*` were deleted, and the file lists what must not return — but preserves `shared/remote-types.ts` as a **frozen contract** (ch. 12). A well-documented boundary explains not just what exists, but **what was removed and why**.

## Chapter checklist

- [ ] Every top-level directory has a one-sentence responsibility (readable by an agent in 10s).
- [ ] The runtime separation is visible in the folder structure (`crates/` vs `packages/` vs `shared/`).
- [ ] No side touches the other's state by shortcut (frontend doesn't touch SQLite; backend doesn't render JSX).
- [ ] The proxy/preview makes clear who hosts whom.
- [ ] Boundary decisions are recorded in ADR, not just in memory or a commit message.
- [ ] What was removed has a record of why **and** what to preserve (frozen contracts).
- [ ] A new agent, reading `AGENTS.md` + `Cargo.toml` + `packages/`, knows where to create each file without asking.
