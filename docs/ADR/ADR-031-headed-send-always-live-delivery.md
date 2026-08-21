# ADR-031: Headed Sessions — Send Always Routes Through Live Delivery

- **Status**: Accepted
- **Date**: 2026-08-21

## Context

A headed session (Claude Code Headed, OpenCode Headed — see `container.rs:1460` / `sessions/mod.rs:266`) runs the real interactive CLI inside a detached tmux session that vibe-kanban attaches a terminal emulator to. Two independent delivery mechanisms already existed for getting text into that live session:

- `send_interactive_input` (`crates/server/src/routes/execution_processes.rs:274`, `POST .../send-input`) — a single-line operator keystroke: rejects newlines/control characters, capped at `MAX_INPUT_LEN`. Built for short answers (e.g. `orchestrator_compactor.rs` uses it to send `/compact`).
- `send_interactive_message` (`crates/services/src/services/container.rs:991`) — a full (possibly multi-line) prompt delivered as a single bracketed-paste block plus Enter. Already wired into the normal follow-up dispatcher (`run_follow_up` / `should_deliver_to_live_session`, `crates/server/src/routes/sessions/mod.rs:160-170,293-334`): when a headed session has a live tmux session, the prompt goes straight into it — busy or idle — with a documented, tested fallback to a fresh `--resume` execution if the tmux session died between the liveness check and delivery ("the prompt is never silently dropped").

Despite the follow-up endpoint already handling this correctly at any time, the workspace chat box's Send button did **not** use it for headed sessions. It used a separate, narrower path (`SessionChatBoxContainer.tsx`'s `headedLive`/`handleSendLive`, calling `send_interactive_input`) gated to **idle only** — mid-turn, Send rendered disabled ("Agent is working…", the `'headed-busy'` status in `packages/ui/src/components/SessionChatBox.tsx`). This meant: (a) full multi-line follow-ups couldn't be composed through Send at all for headed sessions (the single-line endpoint rejects them), and (b) the user couldn't send anything to a busy headed session from the chat box, even though the CLI running inside it — and the backend's own follow-up endpoint — both already support that.

This surfaced from a user question about Claude Code's own CLI behavior (typing/queueing the next message while a turn is still streaming, without waiting) and a request to bring the same capability into the workspace UI.

## Decision

`SessionChatBoxContainer.tsx`'s Send action for headed sessions now always calls the normal `handleSend` (→ `useSessionSend` → `POST /api/sessions/{id}/follow-up`), the same call used for headless sessions — no more branching to `handleSendLive`/`send_interactive_input` from the Send button. The backend's existing live-delivery logic (unchanged by this ADR) decides what happens:

- Live tmux session present → `send_interactive_message` injects the prompt directly, whether the agent is idle or mid-turn.
- No live session (e.g. it died) → falls back to spawning a fresh `--resume` execution, same as any other headless follow-up.

`'headed-busy'` (`packages/ui/src/components/SessionChatBox.tsx`) is kept as a status value but now renders **identically to `'idle'`** — an enabled Send gated only on `canSend` (has content) — instead of a disabled button. It exists solely to drive the working-pulse animation (`showRunningAnimation`); it no longer gates Send. `effectiveStatus` in the container still overrides the base `status` to `'idle'`/`'headed-busy'` whenever there's a live headed process, because the base `isAttemptRunning`-derived `status` would otherwise be stuck on `'running'` (Queue+Stop, no Send) continuously — a headed session's process never exits between turns, so it never satisfies the headless "running → idle" transition the base status logic expects.

`send_interactive_input`/`handleSendLive`'s underlying machinery is untouched and still reserved for short, single-line operator keystrokes elsewhere (approvals go through a wholly separate `useApprovals`/`approveAsync`/`denyAsync` system, unaffected by this change; `/compact` in `orchestrator_compactor.rs` is unaffected).

This only affects Claude Code Headed and OpenCode Headed sessions — `headedLiveProcess` is derived from `getInteractiveConfig(process) != null`, which is `null` for every other executor (Codex, Gemini, Qwen, Cursor, Droid, Amp, Copilot, Antigravity, CCR). Headless sessions are unaffected: their `'running'` status still only offers Queue/Stop, never a live Send, and queued messages remain visibly queued (banner + editor content) until the running process exits.

## Consequences

- Headed-session follow-ups can now be full multi-line messages, sent at any time — matching what a user typing directly into the CLI's own terminal could already do, and closing the gap that prompted this change.
- One less special-cased code path in the chat box: `liveSendError`, `isSendingLiveInput`, and the idle-gated `handleSendLive` were removed; `headedLiveProcess`/`headedLoadingPresent` remain, now used only to pick the animation status.
- **Known, unverified risk**: `send_interactive_message` presses Enter after the bracketed paste. If the underlying CLI's own *native* TUI menu (a y/n or numbered choice the CLI draws itself — distinct from vibe-kanban's tracked `approvalMode`/`askQuestionMode` banner, which is unaffected by this change) happens to be showing when the message lands, the pasted text may have no text field to land in, and the trailing Enter could confirm whatever option is currently selected by default — silently answering a prompt the user never saw, rather than merely failing to deliver the text. This has not been verified either way against Claude Code's or OpenCode's actual bracketed-paste handling while such a menu is active. Flagged in code at the `'idle'`/`'headed-busy'` case in `packages/ui/src/components/SessionChatBox.tsx`; not fixed here.
- No change to headless sessions, to how the two live-injection primitives (`send_interactive_input` vs. `send_interactive_message`) work, or to the approval-answering system.
