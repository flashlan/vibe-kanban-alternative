# ADR-025: Multi-Agent Handoff Pipelines (Plan → Code → Review)

- **Status**: Accepted
- **Date**: 2026-08-20

## Context

In single-developer autonomous workflows, long-running coding sessions handled by a single agent encounter two primary limitations:

1. **Context Window Exhaustion & Rot**: As a single agent progresses through broad exploration, architecture planning, test runs, extensive code edits, and code reviews, its accumulated transcript reaches hundreds of thousands of tokens. This leads to degraded instruction following, subtle hallucination, and high latency.
2. **Model Specialization Mismatch**: Different phases of a feature lifecycle require different model strengths:
   - **Architecture & Planning**: Demands deep reasoning, global dependency awareness, and edge-case anticipation (e.g., Gemini 2.5 Pro / Claude Opus / o3 with high reasoning effort).
   - **Code Implementation**: Demands fast, precise, cost-effective code synthesis, tool execution, and unit test loops (e.g., Claude 3.7 Sonnet / Gemini 2.5 Flash / Qwen Coder).
   - **Code Review & Auditing**: Demands adversarial, independent analysis of the git diff without bias from the authoring agent's intermediate thought process (e.g., Codex / Antigravity Reviewer).

Prior to this decision, pipeline stages (`assets/pipelines/*.toml`) defined ordered prompt fragments, but all stages were executed sequentially within a single agent's execution process.

## Decision

We introduce **Multi-Agent Handoff Pipelines**:

### 1. Per-Stage Executor & Model Bindings in Pipelines (`.toml`)
Each stage in a pipeline definition can optionally bind a specific executor, model, and reasoning effort:

```toml
name = "Swarm Multi-Agent"
description = "Plan (Gemini Pro) → Implement (Claude Sonnet) → Review (Codex)"

[[stage]]
id = "plan"
label = "Architecture & Spec"
executor = "antigravity"
model = "gemini-2.5-pro"
reasoning_effort = "high"
prompt = "Analyze the codebase, mem0 memories, and requirements. Create SPEC.md and IMPLEMENTATION_PLAN.md with task checkboxes."

[[stage]]
id = "implement"
label = "Code Implementation"
executor = "claude"
model = "claude-3-7-sonnet"
prompt = "Read IMPLEMENTATION_PLAN.md and implement each task, running tests and committing incrementally."

[[stage]]
id = "review"
label = "Independent Code Review"
executor = "codex"
prompt = "Review the branch diff against base, check for security and logic regressions, and verify acceptance criteria."
```

### 2. Workspace as the Single Source of Truth
All agents assigned to a card operate inside the **same linked Git worktree** (`.vibe-kanban/worktrees/<card-id>`). Handoff context is anchored in durable, structured files:
- `SPEC.md` & `IMPLEMENTATION_PLAN.md` at the workspace root.
- Persistent project memory via `mem0` (`VK-MEMORY:` facts and `memory_search` recall).
- Git branch history (staged and committed diffs).

### 3. Clean Context Bridging & Handoff Watchdog
When an agent finishes its designated stage (signaled by `VK-PIPELINE-STAGE: <next>` or task completion):
- The backend cleanly terminates the current agent's process.
- The next stage's agent is spawned with a fresh context window, receiving the target stage prompt, the path to `IMPLEMENTATION_PLAN.md`, and the `mem0` recall block.
- Raw transcript history from earlier stages is omitted from the incoming agent's prompt, preventing context rot while guaranteeing that all architectural decisions and plans are preserved.

### 4. UI Timeline & Stepper
The card and workspace views display a multi-agent stepper indicating which model/agent executed each stage and which agent is actively driving the workspace.

## Consequences

- **Higher Quality & Robustness**: Complex tasks leverage specialized models for planning, coding, and review without hitting single-agent context degradation.
- **Cost Efficiency**: High-reasoning tier models are used only during planning, while faster models execute implementation.
- **Backward Compatibility**: Existing pipelines without `executor`/`model` fields continue to run under the workspace's default executor.
