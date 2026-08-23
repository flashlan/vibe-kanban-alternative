//! Wire shape for `GET /api/general-rules/resolve`, consumed by the
//! `get_rules` MCP tool. Global-only (no per-project stack, unlike
//! `ResolvedOrchestratorPromptResponse` in `project.rs`) — mirrors the
//! `commit_reminder_prompt`/`pr_auto_description_prompt` precedent, which
//! are also global `Config` fields, not per-project.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Resolved general-rules pre/post text — either the user's custom override
/// (`Config.general_rules_pre`/`_post`) or the built-in default.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct ResolvedGeneralRules {
    pub pre: String,
    pub post: String,
}
