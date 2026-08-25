# ADR-026: Autonomous Swarm PM Agent, Role Roster & Manual Review Gate

- **Status**: Accepted
- **Date**: 2026-08-20

## Context

Following [ADR-025](/ADR/ADR-025-multi-agent-handoff-pipelines) (Multi-Agent Handoff Pipelines), large-scale user requests require two critical capabilities for a true autonomous single-developer workflow:

1. **Autonomous Task Decomposition (Hierarchical Sub-Issues & Swarm PM)**:
   - When a developer submits a complex high-level feature (e.g. "Implement GitHub OAuth login with JWT sessions and user profile UI"), no single prompt can safely implement the full stack in one run.
   - An Autonomous **PM / Architect Agent** (powered by high-reasoning Gemini 2.5 Pro / Antigravity with `mem0` RAG) must ingest the request, query project context, create an Epic parent card, and use MCP tools to automatically file granular, atomic child **Sub-Issues** on the Kanban board with clear Acceptance Criteria and assigned specialist roles (`Planner`, `Coder`, `QA Tester`, `Reviewer`).

2. **The "Manual Review / Dev Server" Gate (Human-in-the-Loop Verification)**:
   - Automated tests alone cannot catch visual glitches, unexpected UX flows, or missing edge cases.
   - Before an agent or pipeline is allowed to squash-merge changes into the base branch and mark the card as `DONE`, the pipeline must pause at an explicit **Manual Review Gate**:
     - The workspace launches its local dev server / build script (via the preview proxy on port `3003` or direct localhost).
     - The developer is presented with the running app preview, the interactive diff summary, and review action buttons (`[Approve & Merge]` / `[Request Changes]`).
     - Only upon explicit developer approval does the final stage proceed to squash-merge and mark the card `DONE`.

## Decision

### 1. The Autonomous PM / Swarm Orchestrator Role
We define a specialized PM Agent persona equipped with:
- **MCP Kanban Tools**:
  - `create_issue` / `create_sub_issue(parent_id, title, description, priority, stage_pipeline)`
  - `update_issue(status_id, ...)` (moving cards across columns: `Todo` ➔ `In Progress` ➔ `In Review` ➔ `Done`)
  - `list_issues` / `get_issue` (tracking child progress on the Sub-Board)
- **Context & RAG**:
  - `mem0` memory recall for architecture decisions and developer preferences.
  - Read-only codebase indexing to understand current schema and routing before filing sub-tasks.

### 2. Specialized Agent Role Roster

| Role | Executor / Model | Primary Responsibility |
| :--- | :--- | :--- |
| **🧠 PM / Architect** | `antigravity` (`gemini-2.5-pro`) | High-level analysis, RAG recall, sub-issue decomposition. |
| **📚 Researcher** | `agy-research` | Dependency audits, API docs, library compatibility lookups. |
| **💻 Coder** | `claude` (`claude-3-7-sonnet`) | Surgical code edits, unit test implementation per `SPEC.md`. |
| **🧪 QA / Tester** | `local-executor` / `test-runner` | Executes `cargo test`, `pnpm test`, dev server compilation checks. |
| **🔍 Reviewer** | `codex` / `codex-review` | Static analysis, adversarial diff inspection, security audit. |

### 3. The "Manual Review Gate" Pipeline Stage
We introduce a first-class `manual_review` stage type in pipeline definitions:

```toml
[[stage]]
id = "manual-review"
label = "4. Manual Review & Live Preview (Dev Server)"
manual_approval = true
launch_preview = true
prompt = "The implementation and tests are complete. Boot the workspace dev server, output the preview URL, and wait for human operator approval before proceeding to merge."

[[stage]]
id = "merge"
label = "5. Squash-Merge & Done"
default_enabled = true
prompt = "Squash-merge this card's branch into the base branch and transition the card to DONE."
```

When `manual_approval = true`:
1. The backend pauses execution and emits an approval request of kind `human_review` / `manual_review`.
2. The UI renders an actionable banner with:
   - 🌐 **Live Preview Link**: Direct access to the running workspace port (`http://localhost:3003`).
   - 📑 **Diff Inspector**: Side-by-side git diff view.
   - 🔘 **Action Controls**: `[Approve & Merge to Done]` or `[Request Changes with Feedback]`.
3. If approved, the pipeline automatically resumes to execute the final squash-merge and marks the parent card as `Done`.

## Consequences

- **Developer Superpower**: A solo developer can dispatch an entire feature epic to the swarm and only step in at the final preview gate to inspect the live running app and approve the merge.
- **Context Hygiene**: Sub-issues keep each model's context clean and focused on a single responsibility.
- **Zero Accidental Merges**: No code hits the main branch without passing both automated checks AND the developer's live preview approval.
