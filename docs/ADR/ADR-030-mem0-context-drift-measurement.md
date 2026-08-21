# ADR-030: Measure Context Loss/Drift Across mem0 Stage Handoffs

- **Status**: Accepted
- **Date**: 2026-08-21

## Context

ADR-028 made mem0 recall/save agentic: each pipeline stage calls `memory_search` (scoped, ranked, capped to `limit`) before starting and `memory_save` after finishing. That ranking/cap is exactly where two failure modes can happen at a stage handoff, and neither was observable before this change:

- **Loss**: a fact stage N saved for the current task doesn't make it into stage N+1's top-`limit` results, crowded out by unrelated memory already in the repo's shared store.
- **Drift**: unrelated ("noise") memory outranks the actual signal, so the next stage recalls facts that don't belong to the task in front of it.

Before this ADR, `crates/mcp/src/task_server/tools/mem0.rs::memory_search` computed a `score` per hit (used internally to sort/dedupe/cap — `Mem0VectorHit.score`) but discarded it before returning to the caller and before logging; the only signal available was `tracing::info!(hits = memories.len())` — a count, not a relevance measure. There was no test anywhere that exercised recall quality; `mem0-vk/test/test.ts` is a wiring smoke test (does the endpoint respond, does storage/search round-trip work) with a positional character-histogram stub embedding that has no similarity structure — two different strings embed near-arbitrarily relative to each other, so it could never have caught a ranking regression.

Note: `SPEC.md`/`IMPLEMENTATION_PLAN.md` (the raw-text handoff artifact in the Swarm pipeline, see `assets/pipelines/swarm-multi-agent.toml`) is read whole by the next stage — nothing is ranked or capped, so there's no analogous "loss" to measure there; the only handoff channel with a structural mechanism for dropping content is mem0's ranked/capped `memory_search`.

## Decision

**1. Surface the relevance signal that already existed.** `memory_search` in `crates/mcp/src/task_server/tools/mem0.rs` now tracks the scores of the hits that survive dedup/cap and logs `top_score` and `avg_score` alongside the existing `hits` count in the `tracing::info!(target: "mem0", ...)` line. No change to the tool's return shape (`memories: Vec<String>`) — this is observability only, not a new agent-facing contract.

**2. A structured-diff regression test, in `mem0-vk/test/`** (not runtime code) — `context-drift.test.ts`:
   - Simulates a handoff: stores 2 "signal" facts scoped to a task (a distinctive marker token), then stores a pool of unrelated "noise" facts (simulating a repo's accumulated project memory), then searches with a task-scoped query.
   - Produces a structured diff: `retrieved` (id, score, content, whether each hit is signal or noise), `hits`/`missed` (which signal facts made it into the top-`limit`), `recall`, `noise_intrusions`.
   - Asserts a regression floor: recall must be 1 and no noise fact may outrank a signal fact for this scenario. A future change to embedding, ranking, dedup, or cap logic that quietly degrades recall now fails CI/local test runs instead of silently degrading production handoffs.
   - Required fixing the stub embedding first: `test/harness.ts` (new — factored out of `test.ts`, which now imports it) replaced the positional character-histogram `stubEmbedding` with a deterministic bag-of-words hash (lowercased words → hashed bucket, counted, L2-normalized). The old stub had no similarity structure at all — texts sharing words did not reliably score higher than unrelated texts — so it could not support any recall assertion. Verified: with the histogram stub, the drift test's signal facts scored *below* random noise (recall 0); with the bag-of-words stub, signal scored 0.674 vs. 0 for disjoint-vocabulary noise (recall 1). `test.ts`'s existing 36 checks still pass unchanged against the new stub — none of them depend on ranking precision, only on results existing.

## Consequences

- `mem0-vk`'s logs now carry a per-call relevance signal (`top_score`, `avg_score`), cheap to scrape into a dashboard or alert on later if desired — not done here.
- `npm run test:drift` (in `mem0-vk/`) is a repeatable regression guard for recall quality; `npm test` remains the fast wiring smoke test. Both share `test/harness.ts`.
- Trade-off: the bag-of-words stub embedding validates the mem0-vk scoring/ranking *mechanism* (sort, dedupe, cap), not the semantic quality of whatever real embedding backend is configured in production (sentence-transformers, OpenAI, etc.) — it cannot catch a production embedding model regressing.
- Trade-off: `SPEC.md`/`IMPLEMENTATION_PLAN.md` raw-text handoff still has no drift/loss measurement — by design there's nothing to measure there (the whole file is read, nothing ranked or dropped). If that changes (e.g. the file is summarized or truncated before handoff), this ADR's approach would need a counterpart there too.
