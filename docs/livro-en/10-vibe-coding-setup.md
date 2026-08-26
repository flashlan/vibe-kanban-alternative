# Chapter 10 — The Vibe Coding Setup

> **Principle:** context is the AI's source code. Before writing a line of code, write the files that tell a machine how the project works.

## The problem this chapter solves

A coding agent arrives at your repo like a new dev on day one: it doesn't know where anything lives, which commands run, or what must never be touched. A human would ask; the agent **assumes** — and assuming wrong is expensive. The Vibe Coding Setup is the documentation that turns assumption into reading.

## The context files — one canonical, the rest point

The ecosystem converged on context files at the repo root, read automatically by tools:

| File | Who reads |
| --- | --- |
| `AGENTS.md` | Open standard (agents.md): OpenCode, Codex, Cursor and any compatible tool |
| `CLAUDE.md` | Claude Code |
| `.clinerules` | Cline |
| `.cursorrules` / `.cursor/rules/` | Cursor (legacy; now `.cursor/rules/`) |

You don't need all. You need **one canonical and pointers**. Here: `AGENTS.md` at the root is canonical (written for "every agent that works here — Claude Code, OpenCode, Codex, Cursor"), and `docs/CLAUDE.md` exists as a bridge. Maintaining two files with the same content invites drift; keep one and reference.

## Anatomy of a working AGENTS.md (with real lines)

The root `AGENTS.md` is a good specimen. Section by section, and why each:

### 1. Identity in one sentence

> "Aurapunk IDE — the independent, self-hosted fork of vibe-kanban (BloopAI), built for a single-developer process (no team, no cloud, no auth). It is based on the Vibe Kanban Indie fork (dexloom)…"

That line alone stops an agent from "helping" by reintroducing cloud code — and there's an explicit section listing deleted crates (`crates/remote`, `crates/relay-*`) with "do not reintroduce".

### 2. Live work state — the Board Status

```
## Board Status (agent-maintained checklist)
- [x] Done — Add image attachment to create issue dialog (`vk/5f5b-...`)
- [ ] In Review — Livro Vibe Kanban na Amazon (`vk/1f98-livre-vibo-kanba`)
```

One line per card, with the branch (`vk/xxxx-slug`) so another agent can `git switch` directly. Context isn't only static — it's the current state of in-flight work.

### 3. Interaction protocols — the file becomes a behavior contract

This repo goes beyond documenting: it defines **MCP protocols** the agent must run — fetch the card's pipeline (`get_pipeline`), report stage (`report_pipeline_stage` + `VK-PIPELINE-STAGE: N` line), fetch general rules (`get_rules`). The context file becomes a behavior contract.

### 4. Territory map

"Project Structure & Module Organization": a paragraph per top-level directory (`crates/`, `packages/`, `shared/`, `assets/`…), including the warning "shared/types.ts is generated — don't edit by hand". The agent that reads this doesn't waste time searching or edits the wrong file.

### 5. Canonical commands

Exactly how to run: `pnpm i`, `pnpm run dev`, `pnpm run check`, `cargo test --workspace`, `pnpm run generate-types`, `pnpm run format`. Ch. 13 is all about this.

### 6. Conventions and traps

Style (rustfmt, Prettier 2-space/single-quote/80-col), fixed dev ports (3001/3002/3003), "never commit secrets", "run `pnpm run format` before completing".

### 7. Architectural decisions

It points to `docs/ADR/` as where decisions live — and tells the agent to consult before proposing alternatives.

## Context in layers: one AGENTS.md per scope

- `AGENTS.md` (root) — whole repo.
- `docs/AGENTS.md` — Mintlify writing rules, only for doc editors.
- `packages/local-web/AGENTS.md` — web design system and styling.

An agent editing a React component doesn't need the doc-writing rules; an agent editing docs doesn't need Tailwind conventions. Context per directory avoids diluting what matters — and saves tokens each session.

> **Exercise:** create `AGENTS.md` in your SaaS with 4 sections — identity (1 sentence), map (1 line per folder), commands (`dev`/`check`/`format`) and "what is generated / what never to do". When an agent creates `shared/types.ts` by hand and breaks CI, you'll know the "don't edit by hand" line was missing.

## Reproducible environment

- **Runtime versions:** `package.json` declares `engines: node >= 20, pnpm >= 8` and `packageManager: pnpm@10.13.1`; the Cargo workspace declares `edition = "2024"` and a shared version.
- **Fixed dev ports:** frontend 3001, backend 3002, preview proxy 3003 — documented in `AGENTS.md` and exported by `pnpm run dev`. No agent guesses a port.
- **Secrets out of repo:** `.env` for local overrides; Telegram config in `~/.vibe-kanban/telegram.toml` with a committed example (`automation/telegram.toml.example`).

## Chapter checklist

- [ ] A canonical context file exists at the root (and pointers, not copies, for specific tools).
- [ ] It opens with what the project **is and is not** (includes what was removed and must not return).
- [ ] It lists the exact install/dev/check/test/format commands.
- [ ] It lists what is generated and cannot be hand-edited.
- [ ] Scope-specific context lives in the subfolder's `AGENTS.md`.
- [ ] Runtime versions, package manager and ports are declared.
- [ ] Secret files have a committed example and an ignored real file.
- [ ] A new agent can make its first commit without asking anything — test with `vk/quick`.
