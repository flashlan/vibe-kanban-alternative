# Chapter 13 — The Engineering Loop: CLI and self-correction

> **Principle:** an agent only self-corrects if it can run, fail, read the error and repeat — without asking permission each step. Your job is to make that loop short, legible and surprise-free.

## The loop in one sentence

```
write → run tests/checks → read the error in the log → fix → repeat
```

When the loop is fast, the agent solves 90% alone. When it's slow or illegible, it stops and asks — exactly what the approval system and review alarm try to avoid (ch. 06). This chapter is about making the loop so good that escalation becomes the exception.

> In the AssinaFácil SaaS (ch. 08): each card ends with `pnpm run check` green + Preview validated. If `check` explains the error, the agent fixes alone; if it only says "failed", you intervene. The difference is entirely this chapter.

## The canonical commands (real case)

In `package.json`, the scripts are the spec of the loop. Any agent that reads `AGENTS.md` learns the same sequence:

```bash
pnpm i                                    # install
pnpm run dev                              # web (3001) + backend (3002) with fixed ports
pnpm run check                            # tsc ×3 + cargo check + guards
pnpm run lint                             # ESLint + cargo clippy -- -D warnings (qa-mode)
pnpm run format                           # cargo fmt + Prettier — mandatory before completing
cargo test --workspace                    # Rust tests
pnpm run generate-types                   # regenerates shared/types.ts (ch. 12)
pnpm run prepare-db                       # SQLx offline
```

`pnpm run check` is the guardian: `local-web:legacy-path-guard`, `check:db` (frozen migrations), `local-web:check`, `web-core:check`, `ui:check`, `backend:check`. Each guard has a message teaching the fix — not just "failed".

Rule from `AGENTS.md`: before completing any task, `pnpm run format`. Not politeness — it guarantees `cargo fmt --all` and Prettier won't produce phantom diffs on the next commit.

## Three patterns that make the loop teach

### 1. Guards with actionable messages

`check-migration-frozen.sh` blocks editing a published migration and says why; `check-legacy-frontend-paths.sh` blocks old imports and points to the new path. The agent reading the error knows exactly what to fix.

### 2. Warnings as errors

`backend:lint` runs `cargo clippy --workspace --all-targets --features qa-mode -- -D warnings`. In `qa-mode` nothing passes as a warning — every Clippy complaint breaks CI. The agent leaves no "fix later" debt.

### 3. Logs filterable by crate

In `crates/server/src/main.rs:33`, `EnvFilter` is built per crate from `RUST_LOG`. With `DISABLE_WORKTREE_CLEANUP=1 RUST_LOG=debug cargo watch -w crates -x 'run --bin server'`, the agent reads a filtered log and knows if the error came from db, executor or routing.

## The logs as machine interface (bridge to ch. 14)

The detail that matters most for automation: the same logs a human reads are the interface the pipeline trackers read. In `crates/services/src/services/pipeline_stage.rs` and `review_request.rs`, a `Regex` scans the `MsgStore` (unified stdout of headless and headed runs) for textual markers:

- `VK-PIPELINE-STAGE: N` — which pipeline stage the card is at (`parse_pipeline_stage_marker`, with `has_valid_boundary` for escaped `\n`).
- `VK-REVIEW-REQUEST: <msg>` — the agent asks for human review and fires the sound alarm via `NotificationService`.

The agent doesn't call an API to say "I changed stage"; it **writes a line in the log**. The backend observes the log. This keeps all executors (Claude, OpenCode, Codex…) identical to the orchestrator — none needs special integration. The log is the protocol.

## Fixed dev ports and the predictable error

Frontend `3001`, backend `3002`, preview proxy `3003` — fixed, documented, exported by `pnpm run dev`. When an agent tries to start the dev server inside a workspace and the port is already held by another instance, the error is `AddrInUse` in `crates/server/src/main.rs` — predictable, searchable, fixable by checking `lsof -nP -i :3002 -sTCP:LISTEN` and the `cwd` of the process holding it (ch. 02 §5).

## Chapter checklist

- [ ] Each loop command is in `package.json` with a canonical name (`check`, `lint`, `format`, `dev`).
- [ ] `check` includes guards that explain the error and point at the fix.
- [ ] Lint treats warnings as errors (at least in CI/qa-mode).
- [ ] `format` is mandatory before completing — and documented.
- [ ] Logs are filterable per crate via env variable.
- [ ] Progress/review messages are log lines with stable regex — not per-executor API calls.
- [ ] Dev ports are fixed and the `AddrInUse` error has documented diagnosis.
