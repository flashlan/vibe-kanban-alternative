# ADR-028: Agentic mem0 Recall/Save (Drop Blind Context Injection)

- **Status**: Accepted
- **Date**: 2026-08-20

## Context

The original mem0 integration (see ADR-025) had two mechanisms operating in parallel:

1. **Blind recall**: `start_workspace` called `mem0::recall_memories(user_id)` unconditionally for every workspace, fetching *all* stored memories for the repo (unscoped to the card/issue) and prepending a `## Relevant memories from previous sessions` block to the prompt — regardless of whether the pipeline's "memory" stage was even enabled.
2. **Passive save**: `spawn_memory_tracker` ran on every coding-agent execution process, regex-scanning raw stdout for `VK-MEMORY: <fact>` lines and persisting whatever matched.

In parallel, the "Project memory (mem0)" pipeline stage already instructed the agent to call the `memory_search` MCP tool (scoped by a query naming the files/modules/area the card touches) before starting, and the `memory_save` MCP tool after verifying the work — a targeted, deliberate, tool-mediated flow.

This produced two problems:
- **Waste**: the blind recall dumped the entire project memory set into every prompt, unfiltered by relevance to the card, on every workspace start. Since the block is a stable-sorted-but-growing string, any new memory changes it, and because it sits at the very front of the prompt, that invalidates the prompt-cache prefix for every subsequent workspace start — the opposite of cache-hit-friendly.
- **Redundant/noisy writes**: the passive `VK-MEMORY:` marker scan and the deliberate `memory_save` tool call were two independent paths writing to the same store. The passive path could pick up the marker string anywhere in raw output (a quoted log line, a code block, reasoning trace) without the agent having decided the fact was verified — degrading memory quality for every future recall.

## Decision

Drop both automatic mechanisms. `mem0` interaction is now **exclusively agentic**, via the existing MCP tools (`crates/mcp/src/task_server/tools/mem0.rs`):

- **Recall**: the pipeline's "memory" stage prompt instructs the agent to call `memory_search` with a query scoped to the card's files/modules/area, before starting work. No memory content is ever injected automatically into the prompt.
- **Save**: the same stage prompt instructs the agent to call `memory_save` after the work is verified, for durable, self-contained, verified facts only.

Removed:
- `crates/services/src/services/mem0.rs` (`recall_memories`, `save_memory`, `spawn_memory_tracker`, `parse_memory_marker`) and its call sites in `crates/services/src/services/container.rs::start_workspace` and both `spawn_memory_tracker` call sites (`crates/services/src/services/container.rs`, `crates/local-deployment/src/container.rs`).
- The `## Relevant memories from previous sessions` injected block and the `VK-MEMORY:` marker convention.
- The `memory_recall` MCP tool (`crates/mcp/src/task_server/tools/mem0.rs`). It took no query — it fetched *every* memory for the repo — and nothing prevented an agent from calling it instead of the scoped `memory_search`, silently reopening the same full-dump problem this ADR removes elsewhere. Since nothing in the guided flow needs an unscoped fetch, the tool is gone rather than merely discouraged by prompt text: the cap is structural, not just requested.

Additionally, `memory_search` now takes an optional `limit` (default 5) and the server ranks/dedupes hits by score before truncating client-side to `limit`, so even a loosely-scoped query can't flood the agent's context — the cap holds regardless of what the mem0-vk server itself returns.

Unchanged: the `memory_search` / `memory_save` MCP tools and the pipeline "memory" stage prompt (updated to drop the reference to the now-removed injected block).

## Consequences

- Recall is now scoped to what the agent judges relevant to the current card, instead of a full unfiltered dump — fewer tokens spent, less risk of a stale/irrelevant memory leaking into unrelated work.
- The prompt prefix up to (and including) the pipeline stage instructions is now stable across workspace starts regardless of how many memories exist or how often they change, which is prompt-cache-hit-friendly.
- Save is single-path and deliberate: only what the agent explicitly calls `memory_save` on lands in the store, reducing false/noisy memories.
- Recall is now single-path and capped too: `memory_search` is the only fetch tool, and it can't return more than `limit` (default 5) regardless of query breadth or server behavior.
- Pipelines that don't enable the "memory" stage no longer pay any mem0 cost at all (previously the blind recall ran unconditionally on every workspace regardless of pipeline config).
- Trade-off: there is no more passive fallback for executors that don't reliably invoke MCP tools — if an executor can't call `memory_search`/`memory_save`, that workspace gets no project memory. Acceptable since the fork's supported executors are expected to support MCP tool calling.
- Trade-off: an operator who legitimately wants to dump/audit everything stored for a repo (e.g. a future "browse memories" Settings screen) no longer has an MCP tool for that — they'd hit the mem0-vk REST API (`GET /api/memories/{user_id}`) directly, or a dedicated tool would need to be reintroduced scoped to that specific use case rather than exposed to every coding agent.

## Addendum: multi-repo workspace scoping

`memory_search`/`memory_save` take a single `user_id` (a repo slug) per call — memory is stored per-repository, not per-workspace. Most workspaces have exactly one repo, so the stage prompt's `SCOPING` instruction has the agent call `get_context`, read `workspace_repos[0].repo_name`, and use that as `user_id`.

Some workspaces carry more than one repo (`workspace_repos` is a `Vec<McpRepoContext>` — see `crates/mcp/src/task_server/mod.rs`), e.g. a card doing cross-repo integration work against a related service. Scoping only to `workspace_repos[0]` would leave the agent blind to memory that belongs to that second repo. The stage prompt now instructs the agent: `workspace_repos[0]` stays the *primary* `user_id`, but when a fact or query genuinely concerns a different repo also present in `workspace_repos`, call `memory_search`/`memory_save` again with *that* repo's slug as `user_id`. No new tool or parameter was added — `get_context` already exposes the full repo list, and `memory_search`/`memory_save` already accept an arbitrary `user_id`; the gap was purely in the prompt hardcoding index `0`.
