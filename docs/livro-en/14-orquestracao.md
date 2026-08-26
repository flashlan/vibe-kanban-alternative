# Chapter 14 — Agent orchestration: MCP, pipelines and the alarm

> **Principle:** when the agent itself drives the workflow (create cards, report progress, ask review), the tool stops being passive and joins the loop. Three pieces: an MCP server, TOML pipelines, text markers.

## The executors: a dozen agents, one interface

`crates/executors/src/executors/` has one module per agent — `claude`, `codex`, `gemini`, `opencode`, `cursor`, `amp`, `copilot`, `droid`, `qwen`, `antigravity`, `acp` — plus `qa_mock.rs`, a **fake executor** for tests without tokens. Around them: `approvals.rs` (tool-permission flow), `command.rs`/`env.rs` (process setup), `stdout_dup.rs` (log + UI), `mcp_config.rs` (inject the MCP server).

## The MCP server: the board spoken by agents

The `vibe-kanban-mcp` binary (`crates/mcp/`) exposes the board as MCP tools — the protocol Claude Code, OpenCode etc. already speak. One file per domain in `crates/mcp/src/task_server/tools/`: `issues.rs` (cards), `workspaces.rs`/`sessions.rs` (`run_session_prompt`), `pipeline.rs`/`rules.rs` (`get_pipeline`, `report_pipeline_stage`, `get_rules`), `approvals.rs` (`respond_to_approval`), `mem0.rs` (memory). The card you're reading was run by an agent that called `get_pipeline`, reported `VK-PIPELINE-STAGE` and committed — all via these tools. The management tool and the executor are the same system.

## Pipelines in TOML: process as versioned config

The workflow lives in `assets/pipelines/*.toml` (`quick`, `basic`, `speckit`, `swarm-multi-agent`, `wikillm`, `async-*`). A stage from `quick.toml`:

```toml
[[stage]]
id = "review-manual"
label = "Manual review (alarm)"
default_enabled = false
prompt = "MANUAL REVIEW: stop here and hand the work to the operator..."
```

Each stage is a prompt fragment with `id`, `label`, `default_enabled`. The card loads only a pointer; the content comes from `get_pipeline` when the agent runs — so it enters the agent's window only then, not on every board listing.

## Text markers: the invisible orchestration

Two markers sustain the human↔agent loop, parsed from the `MsgStore`:

- **`VK-PIPELINE-STAGE: N`** → `pipeline_stage.rs`: regex `(?i)VK-PIPELINE-STAGE:\s*(\d+)` with `has_valid_boundary` guard. The last valid marker wins; it persists in `workspaces.current_pipeline_stage` — the card's checklist updates live.
- **`VK-REVIEW-REQUEST: <msg>`** → `review_request.rs`: triggers `NotificationService.notify(...)` — the sound alarm. Idempotent per execution; best-effort (notification failure never blocks work).

The channel is **text in the log with formal grammar**, parsed the same in headless (child stdout) and headed (transcript tail). No executor needs to know the marker — only the service reading the stream does. The log is the protocol.

## Supervision: TUI, Telegram and the orchestrator watchdog

| Piece | Command | Does |
| --- | --- | --- |
| **TUI** | `cargo run -p tui` | Terminal cockpit — workspaces/sessions, live transcripts, approvals inbox (`a`) |
| **Telegram bridge** | `cargo run -p telegram-bridge` | Send-only daemon — approvals to Telegram per worktree (`~/.vibe-kanban/telegram.toml`) |
| **OrchestratorCompactor** | `crates/services/src/services/orchestrator_compactor.rs` | Watchdog: every 60s, if transcript > 400k tokens (or 1h with ≥50k), types `/compact` via tmux keys; 10min cooldown; 3 failures → Telegram |

The TUI and Telegram give a human (or another agent) supervision without watching; the Compactor is the context garbage-collector for long runs.

## Chapter checklist

- [ ] Agents have a tool API for the system that manages them (MCP), and a fake executor for tests.
- [ ] The workflow (pipeline) is versioned config, not scattered code.
- [ ] Progress and human requests are text markers with grammar, parser and tests.
- [ ] The log is the protocol — same in headless and headed.
- [ ] Supervision has a human path (TUI/UI) and a remote path (Telegram), with escalation.
