use rmcp::{
    ErrorData, handler::server::wrapper::Parameters, model::CallToolResult, schemars, tool,
    tool_router,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::McpServer;

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct McpGetRulesRequest {
    #[schemars(
        description = "Workspace ID to resolve project-specific rules for. Optional if running inside that workspace context."
    )]
    workspace_id: Option<Uuid>,
}

/// Wire response — mirrors `api_types::ResolvedGeneralRules`. Kept as a
/// distinct schemars-documented struct (rather than returning the api_types
/// shape directly) so each field carries its own description for the
/// calling agent, matching the `get_orchestrator_prompt`/`get_pipeline`
/// pattern.
#[derive(Debug, Serialize, schemars::JsonSchema)]
struct McpGetRulesResponse {
    #[schemars(
        description = "Always-on guardrails to keep in mind THROUGHOUT this card's work (scoping, recall, project guidelines)."
    )]
    pre: String,
    #[schemars(
        description = "Closing checklist and prohibitions to run once the work is verified, before finishing."
    )]
    post: String,
}

#[tool_router(router = rules_tools_router, vis = "pub")]
impl McpServer {
    /// Resolve general and project-scoped rules: pre/post guidance for any card.
    #[tool(
        description = "Resolve general and project-scoped rules — pre/post guidance for how to work on any card. Call this ONCE at the start of a card's execution: keep `pre` in mind throughout the work, and run through `post` as a checklist right before finishing."
    )]
    async fn get_rules(
        &self,
        Parameters(McpGetRulesRequest { workspace_id }): Parameters<McpGetRulesRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let ws_id = self.resolve_workspace_id(workspace_id).ok();
        let url = match ws_id {
            Some(id) => self.url(&format!("/api/general-rules/resolve?workspace_id={id}")),
            None => self.url("/api/general-rules/resolve"),
        };
        let resp: api_types::ResolvedGeneralRules =
            match self.send_json(self.client.get(&url)).await {
                Ok(r) => r,
                Err(e) => return Ok(Self::tool_error(e)),
            };

        McpServer::success(&McpGetRulesResponse {
            pre: resp.pre,
            post: resp.post,
        })
    }
}
