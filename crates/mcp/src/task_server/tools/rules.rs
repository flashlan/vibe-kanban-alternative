use rmcp::{ErrorData, model::CallToolResult, schemars, tool, tool_router};
use serde::Serialize;

use super::McpServer;

/// Wire response — mirrors `api_types::ResolvedGeneralRules`. Kept as a
/// distinct schemars-documented struct (rather than returning the api_types
/// shape directly) so each field carries its own description for the
/// calling agent, matching the `get_orchestrator_prompt`/`get_pipeline`
/// pattern.
#[derive(Debug, Serialize, schemars::JsonSchema)]
struct McpGetRulesResponse {
    #[schemars(
        description = "Always-on guardrails to keep in mind THROUGHOUT this card's work (scoping, recall)."
    )]
    pre: String,
    #[schemars(
        description = "Closing checklist to run once the work is verified, before finishing."
    )]
    post: String,
}

#[tool_router(router = rules_tools_router, vis = "pub")]
impl McpServer {
    /// No parameters: general rules are global config, not per-project or
    /// per-card (unlike `get_pipeline`/`get_orchestrator_prompt`).
    #[tool(
        description = "Resolve this project's general rules — pre/post guidance for how to work on any card. Call this ONCE at the start of a card's execution: keep `pre` in mind throughout the work, and run through `post` as a checklist right before finishing."
    )]
    async fn get_rules(&self) -> Result<CallToolResult, ErrorData> {
        let url = self.url("/api/general-rules/resolve");
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
