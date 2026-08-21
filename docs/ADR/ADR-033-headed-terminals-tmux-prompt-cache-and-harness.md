# ADR-033: Headed Terminals (tmux), Context Traffic, Prompt Caching & Agent Model Harnesses

- **Status**: Accepted
- **Date**: 2026-08-21

## Context

Coding agents in Vibe-Kanban operate under two distinct execution paradigms:
1. **Headless / Stream API mode** (`ClaudeCode`, `Codex`, `Gemini`, `Opencode`, `Antigravity`, etc.): Process runs directly via subprocess spawn with stdout/stderr pipes, or drives an embedded HTTP/SSE daemon. Logs and tool calls are parsed deterministically and streamed sequentially into `MsgStore`.
2. **Headed / Interactive TUI mode** (`ClaudeCodeHeaded`, `OpencodeHeaded`): The agent runs in an interactive TUI within a detached `tmux` session (`tmux new-session -d`), to which terminal emulators (iTerm2, WezTerm, Terminal.app) attach as viewers.

This document establishes the architectural baseline, identifies current implementation gaps in the codebase, and defines the state-of-the-art model harness roadmap.

---

## 1. Headed vs. Headless Terminal Mechanics

### A. Headed (Tmux Detached + Viewers)
- **Lifecycle:** A detached tmux session (`vk-<exec_id>`) is created via a self-deleting launch script in `temp_dir()` (avoiding tmux's 16 KiB `MAX_IMSGSIZE` IPC cap). Emulators attach via `tmux attach -t vk-<exec_id>`.
- **Output Mirroring:** 
  - `ClaudeCodeHeaded`: Tails Claude's on-disk JSONL transcript (`~/.claude/projects/.../<uuid>.jsonl`).
  - `OpencodeHeaded`: Currently a no-op stub awaiting embedded SSE bridge attachment.
- **Input Injection:** Single-line keys via `tmux_send_keys` (with literal `-l` flag and separate Enter) or multi-line messages via bracketed-paste blocks (`\x1b[200~ ... \x1b[201~\n`).

### B. Headless (Raw Subprocess / Streaming Pipe)
- Process standard streams (`stdin`, `stdout`, `stderr`) are consumed as an append-only token stream.
- Output is sanitized (ANSI control sequences stripped), truncated (retaining head and tail), and serialized into structured tool results or message deltas.

---

## 2. Server/Client Context Flow & Prompt Caching (KV-Cache)

Prompt caching in modern LLM providers (Anthropic Claude, Google Gemini, OpenAI) relies on **deterministic prefix matching**:

$$\text{Cache Hit Condition: } \text{Hash}(\text{Prompt}_{0..k}) \equiv \text{Cached Block}$$

### Key Implications:
1. **Append-Only Monotonicity (Headless):**
   - Each turn appends new messages ($M_{k+1}$) to the immutable conversation prefix ($M_0 \dots M_k$).
   - Yields 90–95% cache hit rates, minimizing TTFT (Time To First Token) and token costs.
2. **Screen Scraping & Headed Pitfalls:**
   - Capturing volatile terminal buffers (`tmux capture-pane`) with animated progress bars or spinners causes prefix churn (*Cache Thrashing*).
   - Dynamic mid-buffer mutations invalidate GPU KV caches, escalating token costs to $\mathcal{O}(N^2)$.
3. **Harness Rule:**
   - Raw terminal scraping must be converted into structured delta events before entering model context.

---

## 3. Codebase Audit & Improvement Roadmap

### Current Fragilities Identified:
1. **OpenCodeHeaded Mirroring:** `attach_detached_tracking_opencode` left `MsgStore` empty.
2. **Blind Startup Delays:** `auto_confirm_headed_startup` used hardcoded `sleep` durations rather than buffer regex detection.
3. **Multi-user Script Security:** Temporary launch scripts in `/tmp` require restrictive POSIX `0o700` permissions.

### Target Architecture (State of the Art):
1. **Virtual Terminal Engine (VTE):** Integrated ANSI virtual matrix (`vt100-rs` / `vte`) for universal headless screen scraping.
2. **Real-time KV-Cache Telemetry:** Expose cache hit ratio ($\frac{\text{cache\_read}}{\text{total\_input}}$) and eviction alerts on the kanban card timeline.
3. **Event-driven Prompt Confirmation:** Replace timing delays with active regex monitoring of tmux pane buffers.

---

## Consequences

- Formulates clear architectural guidelines for future agent executors (Qwen, Copilot, Droid, Custom LLM Harnesses).
- Prevents cache invalidation patterns and sets clear performance metrics for terminal I/O streaming.
