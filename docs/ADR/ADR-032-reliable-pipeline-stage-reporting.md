# ADR-032: Reliable Pipeline Stage Reporting (MCP Tool)

- **Status**: Accepted
- **Date**: 2026-08-21

## Context

A card's `## Pipeline` block (composed by `cardPipeline.ts`) instructs the execution agent to output a plain text line — `VK-PIPELINE-STAGE: N` — as it begins each numbered stage. `crates/services/src/services/pipeline_stage.rs` watches the execution's raw stdout stream for that marker and persists the most recent value onto `workspaces.current_pipeline_stage`, which the "Pipeline progress" checklist on the card renders as live "stage N of M" progress.

This is the only mechanism that advances the checklist. It depends entirely on the model choosing to narrate that exact line amid everything else it's doing (reading code, editing files, running tests). In practice some agents implement (and even merge) a card's work correctly without ever emitting the marker — the checklist then sits stuck at "not started yet" indefinitely, with no signal to the operator that this is a known, harmless tracking gap rather than the pipeline having never run. This surfaced directly: an operator running the Quick pipeline saw exactly this — work fully implemented, checklist at 0/3 — and reasonably read it as a system bug, since nothing in the UI distinguishes "never started" from "started but under-reported."

## Decision

Add `report_pipeline_stage` as a real MCP tool (`crates/mcp/src/task_server/tools/workspaces.rs`, part of `workspaces_tools_router`), backed by a new `POST /api/workspaces/{id}/pipeline-stage` endpoint (`crates/server/src/routes/workspaces/core.rs::report_pipeline_stage`). Both the tool and the endpoint take a `stage: i64` and write straight to `Workspace::set_current_pipeline_stage` — the same column the log-marker tracker already writes, so either signal keeps the UI's progress accurate and neither needs to know about the other.

`cardPipeline.ts`'s `ORDER_INSTRUCTION` now asks the agent to do **both** per stage: call `report_pipeline_stage` (primary — a tool call the agent is explicitly instructed to make doesn't depend on it choosing to narrate free text) and still output the `VK-PIPELINE-STAGE: N` line (secondary — kept as a no-cost fallback for the rare case a tool call doesn't land, e.g. an executor with degraded MCP connectivity mid-run).

The tool is excluded from `orchestrator_mode_router()` (`router.remove_route("report_pipeline_stage")`, alongside the existing `list_workspaces`/`delete_workspace` removals): the orchestrator spawns and manages sessions but doesn't itself execute a card's pipeline stages, so it has no stage to report.

## Consequences

- Pipeline progress tracking no longer depends solely on an agent's willingness to narrate a tracking line — a tool call is a much stronger instruction-following signal than free text competing with the actual work.
- The old log-marker tracker (`pipeline_stage.rs`) is unchanged and still runs; this is additive, not a replacement, so no regression risk for the existing path.
- Requires a backend restart to take effect (Rust change, no hot-reload) and only applies to workspaces created after that restart — an already-running MCP session doesn't pick up a newly added tool.
- Trade-off: still no distinction in the UI between "never started" and "started but both signals missed" — a genuinely silent failure (both the tool call and the text marker skipped) still looks identical to "not started yet". Not addressed here; would need the UI to also consider execution status (e.g. "finished but stage never advanced" as a distinct state) if this keeps recurring.
