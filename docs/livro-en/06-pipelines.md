# Chapter 6 — Pipelines in practice

> **Principle:** a pipeline is a recipe the agent follows without you repeating yourself. The card moves; you watch.

## What a pipeline is

A pipeline is a TOML file in `assets/pipelines/*.toml` that lists **stages** with prompts. When a card uses a pipeline, the agent executes stage by stage and reports progress via `VK-PIPELINE-STAGE: N`. The pipeline is the "how" between your spec (the card) and the done state.

## The 9 recipes available

| Pipeline | Shape | Use |
| --- | --- | --- |
| `quick` | implement → verify → manual review (alarm) | Trivial cards; the default |
| `basic` | spec → implement → verify → review | Small features |
| `speckit` | generate SPEC.md → plan → implement | When spec must be written first |
| `swarm-multi-agent` | orchestrate subagents | Parallel fronts of one epic |
| `wikillm` | doc-heavy loop | Writing/explaining tasks |
| `async-*` (variants) | headless, no per-step prompts | Background runs |

## Anatomy of `quick.toml`

```toml
[[stage]]
id = "implement"
label = "Implement directly"
default_enabled = true
prompt = "Implement the card. Run pnpm run check. Report VK-PIPELINE-STAGE: 1."

[[stage]]
id = "review-manual"
label = "Manual review (alarm)"
default_enabled = false
prompt = "MANUAL REVIEW: stop here and hand the work to the operator..."
```

Each stage is a **prompt fragment** with `id`, `label` and `default_enabled`. The card carries only a **pointer** to the pipeline (`<!-- vk:pipeline:start -->`); `get_pipeline` resolves the heavy content at run time — so the prompt enters the agent's window only when the card runs, not on every board listing.

A **tripwire** example: `quick.toml` escalates trivial → light when a condition hits, via the `VK-ESCALATE` marker — letting a cheap card ask for a human only when it truly needs one.

## How progress shows up

As the agent advances, it writes `VK-PIPELINE-STAGE: N` to the log. The service `crates/services/src/services/pipeline_stage.rs` parses it (regex with boundary guard) and persists `workspaces.current_pipeline_stage`. The card's progress checklist updates live in the UI — and you see the number in **Logs** (ch. 04).

> **5-minute exercise:** open a card, assign the `quick` pipeline, dispatch the agent, and watch `VK-PIPELINE-STAGE: 1` then `2` appear in the Logs panel. That single line is the whole orchestration contract.

## Chapter checklist

- [ ] I know a pipeline is a TOML recipe with stages and prompts.
- [ ] I can name at least 4 of the 9 recipes and when to use them.
- [ ] I understand the card carries only a pipeline pointer, not the prompt.
- [ ] I can read `VK-PIPELINE-STAGE: N` in the Logs panel and know what it means.
- [ ] I know the `review-manual` stage raises `VK-REVIEW-REQUEST` (the alarm).
