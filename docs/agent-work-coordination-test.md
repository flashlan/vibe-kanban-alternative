# Agent Work Coordinator: Two-Agent Validation

This runbook validates the automatic declaration hook, the Active agents UI,
semantic dependency warnings, and the database-backed Integration Guard.

## Preconditions

- Start the local application with `pnpm run dev`.
- Use a repository with two independent workspaces attached to the same repo.
- Configure two real coding-agent profiles with valid credentials.

## Scenario

1. Start Agent A in Workspace A with a prompt that changes a named function.
2. Before Agent A edits, confirm the workspace activity endpoint contains an
   active declaration owned by its execution process. The declaration may have
   an empty scope for the first moment; this is the automatic pre-edit hook.
3. Confirm the right sidebar shows **Active agents** and Agent A's intent.
4. Ask Agent A to call `declare_agent_work` with its files, changed symbols,
   and any APIs/modules it depends on.
5. Start Agent B in Workspace B on the same repository and confirm both
   declarations remain visible through the API and the UI.
6. Give Agent B a different file but declare a dependency on a symbol Agent A
   changes. Confirm the MCP response reports a `semantic` conflict.
7. Attempt to merge Workspace A while Agent B's declaration is active.
   Confirm the merge returns `agent_work_conflict`, the target branch is not
   changed, and the card does not move to `Done`.
8. Release or finish Agent B's declaration, retry the merge, and confirm the
   Integration Guard allows the merge and releases Workspace A's declaration.
9. Start two merge requests against the same repository at once. Confirm the
   second request waits for the database lease or returns
   `integration_in_progress` after the bounded wait; it must never write over
   the first integration.

## Evidence to capture

- Screenshot of the **Active agents** panel with both agents.
- The `declare_agent_work` response showing `conflict_type: "semantic"`.
- The blocked merge response and unchanged target branch status.
- The successful merge response and released declaration list.
- Logs showing the second integration did not enter Git mutation concurrently.

This is intentionally a manual acceptance test because it exercises the real
executor/MCP process boundary and real UI state. The database and semantic
coordination primitives also have automated Rust tests.
