# ADR-027: Parallel Read Swarm, Linear Code Execution, and Kanban Dependency Blocking

- **Status**: Accepted
- **Date**: 2026-08-20
- **Author**: Antigravity & Lead Developer
- **Supervises**: Multi-Agent Swarm, Pipelines, Issue Relationships, MCP Task Server

---

## Context

In multi-agent systems, naive concurrent code generation across multiple agents touching the same codebase often leads to Git merge conflicts, broken imports, and non-deterministic state. Conversely, restricting an entire task lifecycle to a single model prevents leveraging specialized models for research, scoping, coding, and review.

To provide a robust, solo-developer-friendly workflow for vibe coding, we formalize a 3-tier execution model:
1. **Parallel Read Swarm**: Multi-agent concurrent analysis for research, codebase indexing, memory recall, and planning (read-only, zero file concurrency risk).
2. **Linear Code Execution**: A single specialized coding agent executing the validated specification.
3. **Issue Dependency Blocking**: Sub-issues in Kanban boards modeled with native blocking relationships (`IssueRelationshipType::Blocking`), preventing premature execution of dependent tasks.

---

## Architecture & Workflow

```
┌────────────────────────────────────────────────────────────────────────┐
│ TIER 1: PARALLEL READ SWARM (Concurrent, Read-Only, Zero Git Conflicts)│
│                                                                        │
│   [ 📚 Researcher ]      [ 📂 Codebase Inspector ]   [ 🧠 mem0 RAG ]    │
│            │                        │                       │          │
│            └────────────────────────┼───────────────────────┘          │
│                                     ▼                                  │
│                       [ 📝 Planner (Gemini 2.5 Pro) ]                  │
│                                     │                                  │
│                      Outputs: SPEC.md + PLAN.md                        │
└─────────────────────────────────────┬──────────────────────────────────┘
                                      │
                                      ▼
┌────────────────────────────────────────────────────────────────────────┐
│ TIER 2: LINEAR CODE EXECUTION (Deterministic & Specialized)            │
│                                                                        │
│                       [ 💻 Coder (Claude 3.7 Sonnet) ]                 │
│                                     │                                  │
│                       [ 🔍 Audit (Codex Review) ]                      │
└─────────────────────────────────────┬──────────────────────────────────┘
                                      │
                                      ▼
┌────────────────────────────────────────────────────────────────────────┐
│ TIER 3: REVIEW GATE & SUB-ISSUE DEPENDENCY RESOLUTION                  │
│                                                                        │
│       [ 🚦 Live Preview Dev Server ] ──► [ 👑 Dev Approval & Merge ]   │
│                                                     │                  │
│                                                     ▼                  │
│       Unblocks dependent Kanban cards (IssueRelationshipType::Blocking)│
└────────────────────────────────────────────────────────────────────────┘
```

---

## Detailed Decisions

### 1. Parallel Read Swarm
- Read-only agents (`Researcher`, `Codebase Inspector`, `RAG Recall`) execute concurrently with zero risk of modifying local files.
- The `Planner` aggregates findings into structured artifacts (`SPEC.md` and `IMPLEMENTATION_PLAN.md`) located in the workspace root.

### 2. Linear Code Execution
- The `Coder` stage is executed sequentially on the Git worktree, following the generated `SPEC.md`.
- No competing coding agent modifies the same files concurrently, guaranteeing clean, deterministic diffs.

### 3. Issue Dependency Blocking (`IssueRelationshipType::Blocking`)
- Sub-issues and epic decomposition can establish blocking relationships via `/api/issue-relationships` and MCP tools (`create_relationship`).
- Blocked cards display dependency badges in the Kanban UI and are prevented from premature execution until predecessor tasks reach `Done`.

---

## Consequences

- **Safety & Stability**: Eliminates Git merge conflicts while maintaining the speed of multi-model research.
- **Cost & Token Efficiency**: Read operations utilize fast, lightweight models; complex code implementation is isolated to top-tier reasoning models.
- **Full Operator Visibility**: Developers retain complete visual tracking over sub-tasks, blockers, and preview gates.
