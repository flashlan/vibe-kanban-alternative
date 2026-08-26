use rmcp::{
    ErrorData, handler::server::wrapper::Parameters, model::CallToolResult, schemars, tool,
    tool_router,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::McpServer;

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct McpGetPipelineRequest {
    #[schemars(
        description = "Workspace ID to resolve the pipeline for. Optional if running inside that workspace context."
    )]
    workspace_id: Option<Uuid>,
}

/// One resolved stage. `report_hint` restates the report instruction on
/// EVERY stage (not just once in a preamble) — read it again before
/// finishing each stage, since a single early instruction is easy to lose
/// track of several stages into a long execution.
#[derive(Debug, Serialize, schemars::JsonSchema)]
struct McpPipelineStage {
    index: i64,
    id: String,
    label: String,
    prompt_fragment: String,
    report_hint: String,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
struct McpGetPipelineResponse {
    workspace_id: String,
    /// Empty when the card has no pipeline selected — that's a normal
    /// state, not an error; proceed without a pipeline in that case.
    pipeline_names: Vec<String>,
    instructions: String,
    stages: Vec<McpPipelineStage>,
    executor: Option<String>,
    custom_text: Option<String>,
    current_pipeline_stage: Option<i64>,
}

#[tool_router(router = pipeline_tools_router, vis = "pub")]
impl McpServer {
    /// Card-scoped resolve tool: given a workspace, resolves the pipeline
    /// stages selected on its linked card (from `extension_metadata`, not
    /// the card description text) via the server-side `GET
    /// /api/workspaces/{id}/pipeline/resolve` endpoint — the same source of
    /// truth the frontend's stage-progress UI reads. `workspace_id` is
    /// optional if running inside that workspace context.
    #[tool(
        description = "Resolve this card's selected pipeline stages, in execution order. Call this ONCE at the start of a card's execution, before any edits — the card description only carries a short pointer, not the full stage list. Execute the returned stages in the order given (do not add, skip, or reorder), and follow each stage's `report_hint` (call `report_pipeline_stage` and emit `VK-PIPELINE-STAGE: N`) as you complete it. Empty `stages` means no pipeline is selected on this card — proceed without one."
    )]
    async fn get_pipeline(
        &self,
        Parameters(McpGetPipelineRequest { workspace_id }): Parameters<McpGetPipelineRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let workspace_id = match self.resolve_workspace_id(workspace_id) {
            Ok(id) => id,
            Err(error_result) => return Ok(Self::tool_error(error_result)),
        };
        if let Err(error_result) = self.scope_allows_workspace(workspace_id) {
            return Ok(Self::tool_error(error_result));
        }

        let url = self.url(&format!(
            "/api/workspaces/{}/pipeline/resolve",
            workspace_id
        ));
        let resp: api_types::pipeline::ResolvedPipelineResponse =
            match self.send_json(self.client.get(&url)).await {
                Ok(r) => r,
                Err(e) => return Ok(Self::tool_error(e)),
            };

        // The MCP surface intentionally supports only the lightweight Quick
        // pipeline. Keep the broader pipeline catalog available to the UI,
        // but never expose another pipeline's prompts to coding agents.
        let resp = if resp.pipeline_names.iter().all(|name| name == "Quick") {
            resp
        } else {
            api_types::pipeline::ResolvedPipelineResponse {
                workspace_id: resp.workspace_id,
                pipeline_names: Vec::new(),
                instructions: String::new(),
                stages: Vec::new(),
                executor: None,
                custom_text: None,
                current_pipeline_stage: resp.current_pipeline_stage,
            }
        };

        let stages = resp
            .stages
            .into_iter()
            .map(|s| McpPipelineStage {
                index: s.index,
                id: s.id,
                label: s.label,
                prompt_fragment: s.prompt_fragment,
                report_hint: s.report_hint,
            })
            .collect();

        McpServer::success(&McpGetPipelineResponse {
            workspace_id: workspace_id.to_string(),
            pipeline_names: resp.pipeline_names,
            instructions: resp.instructions,
            stages,
            executor: resp.executor,
            custom_text: resp.custom_text,
            current_pipeline_stage: resp.current_pipeline_stage,
        })
    }
}
