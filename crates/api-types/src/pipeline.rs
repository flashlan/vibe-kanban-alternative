//! Wire shape for `GET /api/workspaces/{id}/pipeline/resolve`. Resolves a
//! workspace's linked issue's selected pipeline stages server-side, so the
//! `get_pipeline` MCP tool and the frontend's stage-progress UI share one
//! source of truth instead of each re-deriving it (the MCP tool from a
//! REST call, the frontend previously by parsing the card description's
//! numbered list, which only works for cards whose description still embeds
//! the full stage text).

use serde::{Deserialize, Serialize};
use ts_rs::TS;
use uuid::Uuid;

/// One resolved stage, 1-based `index` matching what `report_pipeline_stage`
/// expects. `report_hint` restates the report-after-this-stage instruction
/// on EVERY stage (not just once in a preamble) so it survives a long,
/// multi-turn execution without depending on the agent recalling a single
/// early instruction several stages later.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct ResolvedPipelineStage {
    pub index: i64,
    pub id: String,
    pub label: String,
    pub prompt_fragment: String,
    pub report_hint: String,
}

/// Resolved pipeline for a workspace's linked card. All fields are "empty"
/// (no error) when the workspace has no linked issue, or the issue has no
/// pipeline selected — this is a normal state, not a failure.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct ResolvedPipelineResponse {
    pub workspace_id: Uuid,
    pub pipeline_names: Vec<String>,
    pub instructions: String,
    pub stages: Vec<ResolvedPipelineStage>,
    pub executor: Option<String>,
    pub custom_text: Option<String>,
    pub current_pipeline_stage: Option<i64>,
}
