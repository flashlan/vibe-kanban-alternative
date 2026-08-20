use rmcp::{
    ErrorData, handler::server::wrapper::Parameters, model::CallToolResult, schemars, tool,
    tool_router,
};
use serde::{Deserialize, Serialize};

use super::McpServer;

/// mem0-vk Docker container (REST + MCP). Defaults to the local mem0 server;
/// override with `MEM0_URL` when it runs elsewhere.
fn mem0_url() -> String {
    std::env::var("MEM0_URL").unwrap_or_else(|_| "http://localhost:8000".to_string())
}

/// Default cap on returned hits when the caller doesn't specify `limit`.
/// Keeps a vague query from flooding the agent's context with loosely
/// related memories — see ADR-028.
const DEFAULT_SEARCH_LIMIT: usize = 5;

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct McpMemorySearchRequest {
    #[schemars(
        description = "What to remember / search for, e.g. 'how does the pipeline stage tracker work?'. Scope it to the specific files/modules/area you're about to touch — never a broad or generic query."
    )]
    query: String,
    #[schemars(
        description = "Repo slug (e.g. 'vibe-kanban-alternative') to scope the search to that project's shared memory"
    )]
    user_id: String,
    #[schemars(
        description = "Max number of memories to return, ranked by relevance. Defaults to 5 — raise it only if you genuinely need more context."
    )]
    #[serde(default)]
    limit: Option<usize>,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
struct McpMemorySearchResult {
    memories: Vec<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct McpMemorySaveRequest {
    #[schemars(
        description = "Self-contained, factual memory to persist. Only save VERIFIED, durable facts — never speculation."
    )]
    content: String,
    #[schemars(
        description = "Repo slug (e.g. 'vibe-kanban-alternative') to scope the memory to that project"
    )]
    user_id: String,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
struct McpMemorySaveResult {
    success: bool,
    stored: bool,
}

#[derive(Debug, Deserialize)]
struct Mem0SearchResponse {
    #[serde(default)]
    vector: Vec<Mem0VectorHit>,
}

#[derive(Debug, Deserialize)]
struct Mem0VectorHit {
    score: Option<f64>,
    payload: Option<Mem0Payload>,
}

#[derive(Debug, Deserialize)]
struct Mem0Payload {
    content: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Mem0SaveResponse {
    ok: Option<bool>,
    stored: Option<Vec<String>>,
}

#[tool_router(router = mem0_tools_router, vis = "pub")]
impl McpServer {
    /// Search the project's shared mem0 memory for facts relevant to a query.
    /// Returns ranked, deduplicated memory contents (best-effort: empty list
    /// when mem0 is unreachable).
    #[tool(
        description = "Search the project's shared memory (mem0) for facts relevant to a query. Use this BEFORE analyzing code or starting work to recall decisions, conventions, and lessons the project already learned. Returns at most `limit` hits (default 5) — a small, cheap call. If the results don't cover what you need, call this again with a narrower or differently-worded query rather than raising `limit`; iterating with a sharper query beats fetching more of a vague one."
    )]
    async fn memory_search(
        &self,
        Parameters(McpMemorySearchRequest {
            query,
            user_id,
            limit,
        }): Parameters<McpMemorySearchRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let limit = limit.unwrap_or(DEFAULT_SEARCH_LIMIT);

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(5))
            .build()
            .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;

        let url = format!("{}/api/search", mem0_url());
        // `limit` is sent as a best-effort hint for servers that honor it;
        // the client-side sort+truncate below is what actually guarantees
        // the cap regardless of server support.
        let body = serde_json::json!({ "query": query, "user_id": user_id, "limit": limit });

        let resp = match client.post(&url).json(&body).send().await {
            Ok(r) if r.status().is_success() => r,
            Ok(r) => {
                return McpServer::err(format!("mem0 search returned status {}", r.status()), None);
            }
            Err(e) => return McpServer::err("mem0 search failed".to_string(), Some(e.to_string())),
        };

        let parsed: Mem0SearchResponse = match resp.json().await {
            Ok(parsed) => parsed,
            Err(e) => {
                return McpServer::err(
                    "failed to parse mem0 search response".to_string(),
                    Some(e.to_string()),
                );
            }
        };

        // Rank by score (higher = more relevant; unscored hits sort last),
        // dedupe by content keeping the best-ranked occurrence, then cap to
        // `limit` — a vague query must not flood the agent's context.
        let mut hits: Vec<Mem0VectorHit> = parsed.vector;
        hits.sort_by(|a, b| {
            b.score
                .unwrap_or(f64::MIN)
                .total_cmp(&a.score.unwrap_or(f64::MIN))
        });

        let mut seen = std::collections::HashSet::new();
        let memories: Vec<String> = hits
            .into_iter()
            .filter_map(|hit| hit.payload?.content)
            .filter(|content| seen.insert(content.clone()))
            .take(limit)
            .collect();

        McpServer::success(&McpMemorySearchResult { memories })
    }

    /// Save a fact to the project's shared mem0 memory. Only persist VERIFIED,
    /// durable, self-contained facts (decisions, conventions, root causes) —
    /// never speculation or unverified claims, so future agents do not pick up
    /// false memories.
    #[tool(
        description = "Save a verified, durable fact to the project's shared memory (mem0). Best-effort: returns stored=false when mem0 is unreachable."
    )]
    async fn memory_save(
        &self,
        Parameters(McpMemorySaveRequest { content, user_id }): Parameters<McpMemorySaveRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(5))
            .build()
            .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;

        let url = format!("{}/api/memories", mem0_url());
        let body = serde_json::json!({ "content": content, "user_id": user_id });

        let resp = match client.post(&url).json(&body).send().await {
            Ok(r) if r.status().is_success() => r,
            Ok(r) => {
                return McpServer::err(format!("mem0 save returned status {}", r.status()), None);
            }
            Err(e) => return McpServer::err("mem0 save failed".to_string(), Some(e.to_string())),
        };

        let parsed: Mem0SaveResponse = match resp.json().await {
            Ok(parsed) => parsed,
            Err(_) => {
                return McpServer::success(&McpMemorySaveResult {
                    success: false,
                    stored: false,
                });
            }
        };

        let stored = parsed.stored.map(|s| !s.is_empty()).unwrap_or(false);
        McpServer::success(&McpMemorySaveResult {
            success: parsed.ok.unwrap_or(true),
            stored,
        })
    }
}
