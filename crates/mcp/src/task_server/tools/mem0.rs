use std::sync::atomic::{AtomicBool, Ordering};

use rmcp::{
    ErrorData, handler::server::wrapper::Parameters, model::CallToolResult, schemars, tool,
    tool_router,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

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

/// Below this cosine-similarity score, a `memory_search` top hit is
/// considered weak enough to plausibly be context drift — used only by
/// [`warn_if_weak_relevance`] (debug builds only, a console warning). The
/// always-on `POST /api/usage/mem0-relevance` report below (feeds Settings →
/// Usage "mem0 recall relevance" in every build) sends the raw `top_score`
/// and does the SAME threshold comparison server-side, in
/// `crates/services/src/services/mem0_relevance.rs::WEAK_RELEVANCE_THRESHOLD`
/// (that crate can't depend on this one, hence two constants — keep both in
/// sync if this changes). Not a tuned/validated threshold, just a starting
/// point; see docs/ADR/ADR-030-mem0-context-drift-measurement.md.
#[cfg(debug_assertions)]
const WEAK_RELEVANCE_THRESHOLD: f64 = 0.3;

/// Debug-build-only development aid: warns when a `memory_search` call's
/// best hit looks weak (see [`WEAK_RELEVANCE_THRESHOLD`]), or when it has
/// hits but literally no score at all (shouldn't normally happen from
/// mem0-vk — still worth flagging if seen). Compiled out entirely for
/// `cargo build --release` (`cfg(debug_assertions)` is false there by
/// default, and this crate/workspace does not override it — see the root
/// `Cargo.toml`'s `[profile.release]`), so it costs nothing and never logs
/// in a shipped binary: purely a local-development signal for context
/// drift, not a production alerting mechanism.
#[cfg(debug_assertions)]
fn warn_if_weak_relevance(user_id: &str, query: &str, hit_count: usize, top_score: Option<f64>) {
    let weak = match top_score {
        Some(s) => s < WEAK_RELEVANCE_THRESHOLD,
        None => hit_count > 0,
    };
    if weak {
        tracing::warn!(
            target: "mem0",
            user_id,
            query,
            hits = hit_count,
            top_score = ?top_score,
            "memory_search: weak top relevance score — possible context drift (debug-build-only check, see ADR-030)"
        );
    }
}

/// Best-effort, fire-and-forget POST of one `memory_search` call's relevance
/// to the main VK server's `POST /api/usage/mem0-relevance` (a separate
/// process from this MCP server — see `crates/server/src/routes/usage.rs`),
/// feeding the Settings → Usage "mem0 recall relevance" panel. Spawned
/// rather than awaited inline: this must never add latency to the tool call
/// it's reporting on, and a failure here (VK API down, network hiccup) must
/// never surface to the agent — this is purely observability, unlike
/// `memory_search`'s own graceful-degradation behavior toward mem0 itself.
fn report_mem0_relevance(server_url: String, client: reqwest::Client, top_score: Option<f64>) {
    tokio::spawn(async move {
        let body = serde_json::json!({ "top_score": top_score });
        if let Err(e) = client.post(&server_url).json(&body).send().await {
            tracing::debug!(
                target: "mem0",
                error = %e,
                "failed to report mem0 relevance to VK API (non-fatal, purely observability)"
            );
        }
    });
}

/// mem0 is an OPTIONAL dependency (a separate Docker container the user may
/// not have running). A connection failure — as opposed to mem0 responding
/// with an error — almost always just means "not installed/not started",
/// not a bug worth repeating on every single tool call across a whole
/// session. Log it once per process at `warn`, then drop to `debug` for the
/// rest of that process's lifetime; every call still gracefully degrades
/// (empty search results / stored=false) rather than failing the tool call,
/// so an agent is never blocked by mem0 being absent.
static MEM0_UNREACHABLE_WARNED: AtomicBool = AtomicBool::new(false);

fn note_mem0_unreachable(op: &str, user_id: &str, error: &str) {
    if MEM0_UNREACHABLE_WARNED
        .compare_exchange(false, true, Ordering::Relaxed, Ordering::Relaxed)
        .is_ok()
    {
        tracing::warn!(
            target: "mem0",
            op,
            user_id,
            error,
            "mem0 unreachable — treating as not installed; degrading gracefully for the rest of this session (further occurrences logged at debug)"
        );
    } else {
        tracing::debug!(target: "mem0", op, user_id, error, "mem0 still unreachable");
    }
}

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

/// Cap on `hops` a caller may request. mem0-vk's own `/graph/traverse` caps
/// at 3 server-side too (see `embeddings/app.py`'s `MAX_HOPS`) — this is
/// defense in depth, not the only limit.
const MAX_TRAVERSE_HOPS: u32 = 3;

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct McpGraphTraverseRequest {
    #[schemars(
        description = "Entity name (or substring of one) to start the traversal from — typically a module/file/concept name surfaced by a prior memory_search hit, or a symbol you're about to touch. Matched by substring against node names/descriptions; if nothing matches, the result comes back empty."
    )]
    start: String,
    #[schemars(
        description = "Repo slug (e.g. 'vibe-kanban-alternative') to scope the traversal to that project's shared memory"
    )]
    user_id: String,
    #[schemars(
        description = "How many hops to follow outward from the start node. Defaults to 2; capped at 3 regardless of what you pass."
    )]
    #[serde(default)]
    hops: Option<u32>,
    #[schemars(
        description = "'out' = what depends on / is caused by `start` (follow subject->object edges); 'in' = what `start` depends on (follow object->subject edges); 'both' (default) = either direction. The graph is directed, so 'out' and 'in' are NOT symmetric — pick deliberately based on which direction you actually need."
    )]
    #[serde(default)]
    direction: Option<String>,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
struct McpGraphTraverseResult {
    /// Existing node(s) `start` matched to begin the traversal. Empty means
    /// no match — nothing else in this result is populated.
    matched_start_nodes: Vec<String>,
    /// "name (type): description" per node reached within `hops` steps.
    nodes: Vec<String>,
    /// "subject -[predicate]-> object" per edge among the reached nodes.
    relations: Vec<String>,
    /// True if mem0-vk's own node cap cut the traversal short.
    truncated: bool,
}

impl McpGraphTraverseResult {
    fn empty() -> Self {
        Self {
            matched_start_nodes: vec![],
            nodes: vec![],
            relations: vec![],
            truncated: false,
        }
    }
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

/// Just the fields this file needs from the VK API's
/// `GET /api/workspaces/{id}/git/status` response — the full
/// `RepoBranchStatus` type lives in the `server` crate, which `mcp` doesn't
/// depend on, so this mirrors it locally (same pattern as `Mem0SearchResponse`
/// et al. for mem0-vk's own responses).
#[derive(Debug, Deserialize)]
struct WorkspaceRepoGitStatus {
    repo_id: Uuid,
    head_oid: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Mem0TraverseNode {
    id: String,
    #[serde(rename = "type")]
    node_type: String,
    #[serde(default)]
    description: String,
}

#[derive(Debug, Deserialize)]
struct Mem0TraverseEdge {
    subject: String,
    predicate: String,
    object: String,
}

#[derive(Debug, Deserialize)]
struct Mem0TraverseResponse {
    #[serde(default)]
    matched_start_nodes: Vec<String>,
    #[serde(default)]
    nodes: Vec<Mem0TraverseNode>,
    #[serde(default)]
    edges: Vec<Mem0TraverseEdge>,
    #[serde(default)]
    truncated: bool,
}

#[derive(Debug, Deserialize)]
struct Mem0SaveResponse {
    ok: Option<bool>,
    stored: Option<Vec<String>>,
}

impl McpServer {
    /// Best-effort resolution of the calling workspace's current HEAD commit
    /// for the repo matching `user_id` (a repo slug — see ADR-028's
    /// multi-repo scoping addendum). `None` on ANY failure — no context, no
    /// matching repo, VK API unreachable, no `head_oid` yet (e.g. a fresh
    /// worktree with no commits) — `memory_save` must still succeed without
    /// provenance rather than fail the tool call over this. See
    /// docs/ADR/ADR-030-mem0-context-drift-measurement.md.
    async fn resolve_commit_sha(&self, user_id: &str) -> Option<String> {
        let context = self.context.as_ref()?;
        let repo = context
            .workspace_repos
            .iter()
            .find(|r| r.repo_name == user_id)?;
        let url = self.url(&format!(
            "/api/workspaces/{}/git/status",
            context.workspace_id
        ));
        let statuses: Vec<WorkspaceRepoGitStatus> =
            self.send_json(self.client.get(&url)).await.ok()?;
        statuses
            .into_iter()
            .find(|s| s.repo_id == repo.repo_id)
            .and_then(|s| s.head_oid)
    }
}

#[tool_router(router = mem0_tools_router, vis = "pub")]
impl McpServer {
    /// Search the project's shared mem0 memory for facts relevant to a query.
    /// Returns ranked, deduplicated memory contents. Best-effort: mem0 is an
    /// optional dependency, so any failure (unreachable, bad status,
    /// unparseable response) degrades to an empty list instead of failing
    /// the tool call — a missing/misbehaving mem0 must never block an agent.
    #[tool(
        description = "Search the project's shared memory (mem0) for facts relevant to a query. Use this BEFORE analyzing code or starting work to recall decisions, conventions, and lessons the project already learned. Returns at most `limit` hits (default 5) — a small, cheap call. If the results don't cover what you need, call this again with a narrower or differently-worded query rather than raising `limit`; iterating with a sharper query beats fetching more of a vague one. mem0 is optional — if it isn't running, this returns an empty list rather than an error."
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
                tracing::warn!(
                    target: "mem0",
                    user_id = %user_id,
                    status = %r.status(),
                    "memory_search: mem0 responded with a non-success status; degrading to empty results"
                );
                return McpServer::success(&McpMemorySearchResult { memories: vec![] });
            }
            Err(e) => {
                note_mem0_unreachable("memory_search", &user_id, &e.to_string());
                return McpServer::success(&McpMemorySearchResult { memories: vec![] });
            }
        };

        let parsed: Mem0SearchResponse = match resp.json().await {
            Ok(parsed) => parsed,
            Err(e) => {
                tracing::warn!(
                    target: "mem0",
                    user_id = %user_id,
                    error = %e,
                    "memory_search: unparseable mem0 response; degrading to empty results"
                );
                return McpServer::success(&McpMemorySearchResult { memories: vec![] });
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
        // Scores of the hits that actually made it into `memories`, in rank
        // order — used below to log a drift/relevance signal alongside the
        // hit count (see docs/ADR/ADR-030-mem0-context-drift-measurement.md).
        let mut scores: Vec<f64> = Vec::new();
        let memories: Vec<String> = hits
            .into_iter()
            .filter_map(|hit| {
                let content = hit.payload?.content?;
                Some((content, hit.score))
            })
            .filter(|(content, _)| seen.insert(content.clone()))
            .take(limit)
            .map(|(content, score)| {
                if let Some(s) = score {
                    scores.push(s);
                }
                content
            })
            .collect();

        // Highest-ranked hit's score and the mean over all returned hits —
        // a cheap proxy for how relevant this batch actually was. A low
        // top_score means the agent's next stage is working from weakly
        // related (or no) memory: the raw signal for measuring context
        // drift across a stage handoff.
        let top_score = scores.first().copied();
        let avg_score = if scores.is_empty() {
            None
        } else {
            Some(scores.iter().sum::<f64>() / scores.len() as f64)
        };

        // Debug-build-only: a step up from the info-level score log above —
        // flags a call whose top hit looks weak enough to plausibly be
        // context drift, at `warn` so it stands out in a dev console. Gated
        // on `cfg(debug_assertions)` (true for `cargo build`/`check`/`test`,
        // false for `cargo build --release` — no Cargo.toml override here,
        // see `[profile.release]`), so this never compiles into, runs in, or
        // logs from a release binary: a local development aid, not a
        // production alerting mechanism. See
        // docs/ADR/ADR-030-mem0-context-drift-measurement.md.
        #[cfg(debug_assertions)]
        warn_if_weak_relevance(&user_id, &query, memories.len(), top_score);

        report_mem0_relevance(
            self.url("/api/usage/mem0-relevance"),
            self.client.clone(),
            top_score,
        );

        tracing::info!(
            target: "mem0",
            user_id = %user_id,
            query = %query,
            hits = memories.len(),
            top_score = ?top_score,
            avg_score = ?avg_score,
            "memory_search ok"
        );

        McpServer::success(&McpMemorySearchResult { memories })
    }

    /// Save a fact to the project's shared mem0 memory. Only persist VERIFIED,
    /// durable, self-contained facts (decisions, conventions, root causes) —
    /// never speculation or unverified claims, so future agents do not pick up
    /// false memories. Best-effort: mem0 is an optional dependency, so any
    /// failure degrades to `stored: false` instead of failing the tool call.
    #[tool(
        description = "Save a verified, durable fact to the project's shared memory (mem0). Best-effort: returns stored=false when mem0 is unreachable or misbehaving, rather than an error — mem0 is optional."
    )]
    async fn memory_save(
        &self,
        Parameters(McpMemorySaveRequest { content, user_id }): Parameters<McpMemorySaveRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(5))
            .build()
            .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;

        // Best-effort provenance: tag this fact (and its graph node/edges,
        // mem0-vk-side) with the calling workspace's HEAD commit, so a
        // later staleness check has something to compare against instead
        // of assuming every fact is permanently valid. A failed lookup just
        // omits it — never blocks the save.
        let commit_sha = self.resolve_commit_sha(&user_id).await;

        let url = format!("{}/api/memories", mem0_url());
        let body = serde_json::json!({
            "content": content,
            "user_id": user_id,
            "commit_sha": commit_sha,
        });

        let resp = match client.post(&url).json(&body).send().await {
            Ok(r) if r.status().is_success() => r,
            Ok(r) => {
                tracing::warn!(
                    target: "mem0",
                    user_id = %user_id,
                    status = %r.status(),
                    "memory_save: mem0 responded with a non-success status; degrading to stored=false"
                );
                return McpServer::success(&McpMemorySaveResult {
                    success: false,
                    stored: false,
                });
            }
            Err(e) => {
                note_mem0_unreachable("memory_save", &user_id, &e.to_string());
                return McpServer::success(&McpMemorySaveResult {
                    success: false,
                    stored: false,
                });
            }
        };

        let parsed: Mem0SaveResponse = match resp.json().await {
            Ok(parsed) => parsed,
            Err(e) => {
                tracing::warn!(
                    target: "mem0",
                    user_id = %user_id,
                    error = %e,
                    "memory_save: unparseable mem0 response; degrading to stored=false"
                );
                return McpServer::success(&McpMemorySaveResult {
                    success: false,
                    stored: false,
                });
            }
        };

        let stored = parsed.stored.map(|s| !s.is_empty()).unwrap_or(false);
        let success = parsed.ok.unwrap_or(true);
        if success && stored {
            tracing::info!(target: "mem0", user_id = %user_id, "memory_save ok");
        } else {
            tracing::warn!(
                target: "mem0",
                user_id = %user_id,
                success,
                stored,
                "memory_save returned ok status but did not confirm storage"
            );
        }
        McpServer::success(&McpMemorySaveResult { success, stored })
    }

    /// Multi-hop traversal of the project's mem0 knowledge graph from a
    /// named entity. Complements `memory_search` (semantic similarity to a
    /// query) with actual graph structure — use it once you already have a
    /// specific entity name and want its neighborhood, not just text that
    /// reads similarly. Best-effort, same graceful-degradation contract as
    /// `memory_search`/`memory_save` — mem0/graph absence degrades to an
    /// empty result, never a failed tool call. See
    /// docs/ADR/ADR-030-mem0-context-drift-measurement.md.
    #[tool(
        description = "Traverse the project's mem0 knowledge graph from a named entity (module, file, concept) outward up to `hops` steps (default 2, max 3), following relation edges. Unlike memory_search (semantic similarity to a query), this follows actual graph structure: direction 'out' = what depends on/is caused by `start`; 'in' = what `start` depends on; 'both' (default) = either. Use this once you already have a specific entity name — e.g. from a prior memory_search hit, or a module/file you're about to touch — and want its neighborhood, not just semantically similar text. `start` is matched by substring against node names/descriptions; no match returns an empty result. Optional — mem0/graph absence degrades to empty, not an error."
    )]
    async fn memory_graph_traverse(
        &self,
        Parameters(McpGraphTraverseRequest {
            start,
            user_id,
            hops,
            direction,
        }): Parameters<McpGraphTraverseRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let hops = hops.unwrap_or(2).min(MAX_TRAVERSE_HOPS);
        let direction = direction.unwrap_or_else(|| "both".to_string());

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(5))
            .build()
            .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;

        let url = format!("{}/api/graph/traverse", mem0_url());
        let body = serde_json::json!({
            "start": start,
            "user_id": user_id,
            "hops": hops,
            "direction": direction,
        });

        let resp = match client.post(&url).json(&body).send().await {
            Ok(r) if r.status().is_success() => r,
            Ok(r) => {
                tracing::warn!(
                    target: "mem0",
                    user_id = %user_id,
                    status = %r.status(),
                    "memory_graph_traverse: mem0 responded with a non-success status; degrading to empty result"
                );
                return McpServer::success(&McpGraphTraverseResult::empty());
            }
            Err(e) => {
                note_mem0_unreachable("memory_graph_traverse", &user_id, &e.to_string());
                return McpServer::success(&McpGraphTraverseResult::empty());
            }
        };

        let parsed: Mem0TraverseResponse = match resp.json().await {
            Ok(parsed) => parsed,
            Err(e) => {
                tracing::warn!(
                    target: "mem0",
                    user_id = %user_id,
                    error = %e,
                    "memory_graph_traverse: unparseable mem0 response; degrading to empty result"
                );
                return McpServer::success(&McpGraphTraverseResult::empty());
            }
        };

        tracing::info!(
            target: "mem0",
            user_id = %user_id,
            start = %start,
            hops,
            matched = parsed.matched_start_nodes.len(),
            nodes = parsed.nodes.len(),
            edges = parsed.edges.len(),
            truncated = parsed.truncated,
            "memory_graph_traverse ok"
        );

        McpServer::success(&McpGraphTraverseResult {
            matched_start_nodes: parsed.matched_start_nodes,
            nodes: parsed
                .nodes
                .into_iter()
                .map(|n| {
                    if n.description.is_empty() {
                        format!("{} ({})", n.id, n.node_type)
                    } else {
                        format!("{} ({}): {}", n.id, n.node_type, n.description)
                    }
                })
                .collect(),
            relations: parsed
                .edges
                .into_iter()
                .map(|e| format!("{} -[{}]-> {}", e.subject, e.predicate, e.object))
                .collect(),
            truncated: parsed.truncated,
        })
    }
}
