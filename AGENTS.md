> **vibe-kanban-indie** — the independent, self-hosted fork of vibe-kanban, built for a **single-developer process** (no team, no cloud, no auth). The TUI cockpit (`crates/tui`, `vibe-tui`) and Telegram channel orchestration (`crates/telegram-bridge`) are the control surfaces a solo dev uses to drive a crew of agents.

## Board Status (agent-maintained checklist)

This file (`AGENTS.md` at the repo root) is read by **every** agent that works here — Claude Code, OpenCode, Codex, Cursor, and any other `agents.md`-compatible tool. Keep the checklist below as a shared, at-a-glance snapshot of active kanban work so any agent can see what is planned, in flight, in review, and done **without re-querying the board**.

Update it as you move cards through their lifecycle. One line per active card:

- [ ] **TODO** — <short card title> (`<branch>`)
- [~] **In Progress** — <short card title> (`<branch>`)
- [ ] **In Review** — <short card title> (`<branch>`)
- [x] **Done** — <short card title> (`<branch>`)

Rules:
- Move a card `[ ]` → `[~]` → `[x]` as it advances (TODO → In Progress → In Review → Done).
- Add a line when you start a card; archive/remove completed lines periodically so this stays short.
- Use the branch name (e.g. `vk/xxxx-slug`) so another agent can `git switch` straight to the work.
- This is a lightweight manual convention, not an automated sync — accuracy depends on agents keeping it current.

- [x] **Done** — AGENTS.md board-status checklist (`vk/8dfb-o-agent-md-na-ra`)
- [x] **Done** — Add image attachment to create issue dialog description (`vk/5f5b-feature-adicioan`)
- [x] **Done** — Add urgency and tags buttons to create-issue dialog (`vk/160b-feature-definir`)
- [x] **Done** — Browser cache for workspace conversations to skip re-stream on switch (`vk/f804-poss-vel-cache-d`)
- [x] **Done** — Workspace color setting (sidebar tree tint) (`vk/3585-altra-cor-do-wor`)
- [ ] **In Review** — Livro Vibe Kanban na Amazon — checklist + manuscrito completo (15 caps + apêndice + Agradecimentos, ~1.830 linhas, 12 âncoras) (`vk/1f98-livre-vibo-kanba`)

## Card Pipeline Protocol (MCP)

A card's description now carries only a **compact pointer** to its pipeline, not the full stage list — the heavy content lives behind the `get_pipeline` MCP tool instead, so it doesn't bloat every model call. When a card's description mentions `get_pipeline`:

- Call it (workspace-scoped; `workspace_id` is optional if you're already running inside that workspace) **before any code edits**.
- Execute the stages it returns **in the order given** — do not add, skip, or reorder.
- After completing **each** stage, call `report_pipeline_stage` with that stage's number AND emit the line `VK-PIPELINE-STAGE: N` before moving to the next one — repeat this for every stage, not just the first. (The tool's own response restates this reminder on each stage entry — re-read it as you go, don't rely on having read it once at the start.)
- Empty `stages` in the response means no pipeline is selected on this card — proceed without one.

A card whose description has no such reference has nothing to fetch — proceed normally.

## Project Rules Protocol (MCP)

Unlike the pipeline pointer above, this one is **unconditional** — general project rules apply to every card, so there's no pointer text to look for in the description.

- Call `get_rules` **once, at the start of every card's execution**, before any code edits.
- Keep its `pre` guidance in mind **throughout** the work — it covers always-on guardrails (e.g. which repo's memory to use, when to recall before starting).
- Right before finishing, run through its `post` field as a **closing checklist** (e.g. what to save, and what never to save).

# Repository Guidelines

## Project Structure & Module Organization
- `crates/`: Rust workspace crates — `server` (API + bins), `db` (SQLx models/migrations), `executors`, `services`, `utils`, `git` (Git operations), `api-types` (shared API types), `review` (PR review tool), `deployment`, `local-deployment`, `tui` (terminal cockpit, `vibe-tui` bin), `telegram-bridge` (send-only escalation daemon, `vibe-telegram-bridge` bin).
- `automation/`: Automated-supervision layer (TUI + Telegram bridge + PM agent) — see [`automation/README.md`](automation/README.md). Telegram config lives in `~/.vibe-kanban/telegram.toml` (example: `automation/telegram.toml.example`).
- `packages/local-web/`: Local React + TypeScript app entrypoint (Vite, Tailwind). Shell source in `packages/local-web/src`.
- `packages/web-core/`: Shared React + TypeScript frontend library used by local-web (`packages/web-core/src`).
- `shared/`: Generated TypeScript types (`shared/types.ts`) and agent tool schemas (`shared/schemas/`). Do not edit generated files directly.
- `assets/`, `dev_assets_seed/`, `dev_assets/`: Packaged and local dev assets.
- `npx-cli/`: Files published to the npm CLI package.
- `scripts/`: Dev helpers (ports, DB preparation).
- `docs/`: Documentation files.

### Crate-specific guides
- [`docs/AGENTS.md`](docs/AGENTS.md) — Mintlify documentation writing guidelines and component reference.
- [`packages/local-web/AGENTS.md`](packages/local-web/AGENTS.md) — Web app design system styling guidelines.

## Architecture Decision Records

All ADRs live in **`docs/ADR/`** as `.md` files (numbered, e.g. `ADR-001-modal-system.md`, with `Status`/`Date`/`Context`/`Decision`/`Consequences`). **Highly advisable to maintain documentation here**: when a non-trivial architecture decision is made (new subsystem, refactor, pattern choice, removed feature), record it as an ADR before or right after the implementation, and keep `Status` accurate (`Accepted` vs `Proposed`). Agents should check `docs/ADR/` for prior decisions before proposing alternatives.

## Legacy cloud/remote code

The fork is local-only; the cloud stack has been removed. The following crates were deleted from disk: `crates/remote`, `crates/relay-tunnel`, `crates/relay-hosts`, `crates/relay-webrtc`, `crates/remote-info`. The `remote:*` scripts in `package.json` and the `backend-remote-checks` CI job have been removed as well. Do not reintroduce them.

Note: `shared/remote-types.ts` (historically generated from `crates/remote`) is NOT dead — it is the live wire-contract for the kanban data layer (`providers/remote/*`, `integrations/electric/*`, `lib/electric/*`), used by the local UI in fallback-REST mode. Keep it and its consumers; treat it as a frozen, hand-maintained contract since its generator has been removed.

## Managing Shared Types Between Rust and TypeScript

ts-rs allows you to derive TypeScript types from Rust structs/enums. By annotating your Rust types with #[derive(TS)] and related macros, ts-rs will generate .ts declaration files for those types.
When making changes to the types, you can regenerate them using `pnpm run generate-types`
Do not manually edit shared/types.ts, instead edit crates/server/src/bin/generate_types.rs

## Build, Test, and Development Commands
- Install: `pnpm i`
- Run dev (web app + backend with ports auto-assigned): `pnpm run dev`
- Backend (watch): `pnpm run backend:dev:watch`
- Web app (dev): `pnpm run local-web:dev`
- Type checks: `pnpm run check` (frontend + all backend Rust workspaces) and `pnpm run backend:check` (all backend Rust workspaces in the workspace)
- Rust tests: `cargo test --workspace`
- Generate TS types from Rust: `pnpm run generate-types` (or `generate-types:check` in CI)
- Prepare SQLx (offline): `pnpm run prepare-db`
- Local NPX build: `pnpm run build:npx` then `pnpm pack` in `npx-cli/`
- Format code: `pnpm run format` (runs `cargo fmt` for all backend Rust workspaces + web-core/web Prettier)
- Lint: `pnpm run lint` (runs web/ui ESLint + `cargo clippy` for all backend Rust workspaces)

## Before Completing a Task
- Run `pnpm run format` to format all Rust workspaces and web code.

## Coding Style & Naming Conventions
- Rust: `rustfmt` enforced (`rustfmt.toml`); group imports by crate; snake_case modules, PascalCase types.
- TypeScript/React: ESLint + Prettier (2 spaces, single quotes, 80 cols). PascalCase components, camelCase vars/functions, kebab-case file names where practical.
- Keep functions small, add `Debug`/`Serialize`/`Deserialize` where useful.

## Testing Guidelines
- Rust: prefer unit tests alongside code (`#[cfg(test)]`), run `cargo test --workspace`. Add tests for new logic and edge cases.
- Web app: ensure `pnpm run check` and `pnpm run lint` pass. If adding runtime logic, include lightweight tests (e.g., Vitest) in the same directory.

## Security & Config Tips
- Use `.env` for local overrides; never commit secrets. Key envs: `FRONTEND_PORT`, `BACKEND_PORT`, `HOST`
- Dev ports are fixed: frontend `3001`, backend `3002`, preview proxy `3003`. Dev assets live in `dev_assets/` (seeded from `dev_assets_seed/`).


