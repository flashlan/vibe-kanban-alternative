# ADR-036: Agent Work Coordinator and Integration Guard

- **Status**: Accepted
- **Date**: 2026-08-25

## Context

Multiple coding agents can work in parallel on related workspaces or on
different sessions within the same workspace. Git conflicts are not sufficient
to detect the risk: two agents can change different lines in the same symbol,
module, or responsibility and still produce a semantically incompatible
result.

Mem0 is not suitable as the coordination source. It is asynchronous, optional,
and may contain stale context. Presence and intended work need transactional
state that can be queried by the backend, MCP, and UI.

## Decision

The overall subsystem is named **Agent Work Coordinator**. It has three
responsibilities:

1. **Agent Activity Observer** — exposes active agents, intent, leases, files,
   and symbols to the UI and MCP.
2. **Soft Reservation Manager** — records declarations and reports overlap
   without blindly blocking parallel work.
3. **Integration Guard** — serializes writes to shared target branches,
   validates the branch diff against active sibling declarations, and blocks
   integration when the overlap requires review.

Add short-lived agent work declarations stored in SQLite and scoped to a
workspace. Each declaration contains an execution owner ID, agent label,
intent, expected files, expected symbols, semantic dependencies, optional
execution ID, and a ten-minute lease.

Declarations are soft reservations. The backend reports file and symbol
overlaps and dependency-contract conflicts to the declaring agent and the UI,
but does not block the second agent. A crashed or disconnected executor stops
appearing after its lease expires. Agents can refresh the lease with a
heartbeat and explicitly release it when their work ends.

The executor creates the initial declaration before launching a coding agent;
the MCP server then refines that row before edits with files, symbols, and
dependencies. The first UI surface is a polling activity panel in the
workspace sidebar. Hard exclusive reservations remain separate from the soft
declaration flow. Direct Git integration uses a database-backed Integration
Guard lease, serializes the validation and write sequence across backend
processes, and treats the merge base as the task's original HEAD. If the target
branch advanced, the guard keeps the task diff separate from the target diff
and lets Git perform a three-way squash merge; independent changes therefore
do not require a rebase. Active declarations from sibling workspaces are
compared with the task diff before the write, and a semantic overlap or real
Git conflict returns a structured conflict without moving the card to Done.
Successful integration releases the workspace declarations before the
existing post-merge card transition runs.

Terminal card completion is also guarded at the issue-update boundary. An
agent cannot set a terminal status through `update_issue`; it must call
`complete_workspace_card`, which defers the terminal transition until the
Integration Guard merge succeeds and Mem0 acknowledges a verified durable
summary. The Kanban UI exposes an explicit operator override for intentionally
moving a card without merging, but never performs that transition silently.

## Consequences

- The current agent activity is visible without consulting Mem0.
- Overlap warnings are durable enough to survive UI refreshes and concurrent
  requests, while stale leases are automatically cleaned up.
- Soft reservations preserve parallelism and make the integration/review cost
  explicit instead of silently rejecting useful work.
- An independently advanced target branch no longer forces a rebase: Git can
  combine disjoint changes from the two diffs. Rebase remains available when
  the operator wants to rewrite the task branch or when the changes genuinely
  conflict.
- The current semantic detector is contract-based: it compares declared
  dependencies with changed symbols. It is intentionally conservative and
  does not replace AST analysis or human review.
- The two-agent acceptance flow is documented in
  `docs/agent-work-coordination-test.md`; it requires real executor
  credentials and is separate from the automated Rust tests.
