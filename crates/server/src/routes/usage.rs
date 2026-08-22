//! Aggregated usage data for the Settings → Usage dashboard.
//!
//! Builds per-day activity from `execution_processes` joined with `sessions`
//! (agent breakdown), plus issue progress (created/completed), a per-project
//! summary, and in-memory LLM token + KV-cache telemetry.

use axum::{
    Json, Router,
    extract::State,
    response::Json as ResponseJson,
    routing::{get, post},
};
use deployment::Deployment;
use serde::{Deserialize, Serialize};
use services::services::mem0_relevance::Mem0RelevanceSummary;
use services::services::token_telemetry::TokenTelemetrySummary;
use sqlx::FromRow;
use ts_rs::TS;
use utils::response::ApiResponse;
use uuid::Uuid;

use crate::DeploymentImpl;

/// Default mem0 server base URL; override with `MEM0_URL`.
fn mem0_url() -> String {
    std::env::var("MEM0_URL").unwrap_or_else(|_| "http://localhost:8000".to_string())
}

/// One day of activity for a single agent.
#[derive(Debug, Serialize, TS)]
pub struct DailyAgentActivity {
    /// `YYYY-MM-DD` (local server time).
    pub day: String,
    pub agent: String,
    pub executions: i64,
    /// Total execution time in seconds (completed/failed/killed only).
    pub seconds: i64,
}

/// Issue activity on a single day.
#[derive(Debug, Serialize, TS)]
pub struct DailyIssueActivity {
    pub day: String,
    pub created: i64,
    pub completed: i64,
}

/// Per-project progress summary.
#[derive(Debug, Serialize, TS)]
pub struct ProjectProgress {
    pub project_id: String,
    pub name: String,
    pub total: i64,
    pub done: i64,
    pub open: i64,
}

#[derive(Debug, Serialize, TS)]
pub struct UsageSummary {
    /// Activity over the last 30 days, one row per (day, agent).
    pub activity: Vec<DailyAgentActivity>,
    /// Issue created/completed over the last 30 days.
    pub issues: Vec<DailyIssueActivity>,
    /// Per-project open/done counts.
    pub projects: Vec<ProjectProgress>,
    /// Total executions + duration across the whole window.
    pub total_executions: i64,
    pub total_seconds: i64,
    /// mem0 extraction-model token usage (best-effort; empty when mem0 is down).
    pub mem0_tokens: Mem0TokenUsage,
    /// `memory_search` recall relevance, day-bucketed — reported live by the
    /// `vibe_kanban_mcp` process via `POST /api/usage/mem0-relevance` (a
    /// separate process from this server, so this can't be read from a
    /// shared in-process struct the way the rest of `UsageSummary` is; see
    /// docs/ADR/ADR-030-mem0-context-drift-measurement.md). In-memory only —
    /// resets on server restart.
    pub mem0_relevance: Mem0RelevanceSummary,
    /// LLM token + KV-cache telemetry, day-bucketed per agent — reported
    /// via `POST /api/usage/token-telemetry`. In-memory only — resets on
    /// server restart.
    pub token_telemetry: TokenTelemetrySummary,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, TS)]
pub struct Mem0TokenUsage {
    pub days: Vec<Mem0TokenDay>,
    pub providers: Vec<Mem0TokenProvider>,
    pub total: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, TS)]
pub struct Mem0TokenDay {
    pub day: String,
    pub prompt: i64,
    pub completion: i64,
    pub total: i64,
    #[serde(default)]
    pub providers: Vec<Mem0TokenProvider>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, TS)]
pub struct Mem0TokenProvider {
    pub provider: String,
    pub model: String,
    pub prompt: i64,
    pub completion: i64,
}

/// Body-free result of `POST /api/usage/re-extract`.
#[derive(Debug, Serialize, Deserialize, TS)]
pub struct ReExtractResponse {
    pub ok: bool,
    pub scanned: i64,
    pub updated: i64,
    pub entities: i64,
    pub relations: i64,
}

/// mem0 runtime config (sanitized — keys never leave the mem0 container).
#[derive(Debug, Clone, Serialize, Deserialize, Default, TS)]
pub struct Mem0Config {
    pub ok: bool,
    pub provider: String,
    pub graph_enabled: bool,
    #[serde(default)]
    pub graph_url: String,
    #[serde(default)]
    pub collection: String,
    #[serde(default)]
    pub providers: std::collections::HashMap<String, Mem0ProviderCfg>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, TS)]
pub struct Mem0ProviderCfg {
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub model: String,
    #[serde(default)]
    pub has_key: bool,
}

/// Body sent by Settings → Memory to update the mem0 runtime config.
#[derive(Debug, Clone, Deserialize, TS)]
pub struct UpdateMem0ConfigRequest {
    #[serde(default)]
    pub provider: Option<String>,
    #[serde(default)]
    pub graph_enabled: Option<bool>,
    #[serde(default)]
    pub providers: Option<std::collections::HashMap<String, Mem0ProviderPatch>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, TS)]
pub struct Mem0ProviderPatch {
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
    /// New key to set. Omit/empty to keep the existing (masked) key.
    #[serde(default)]
    pub key: Option<String>,
}

pub fn router() -> Router<DeploymentImpl> {
    Router::new()
        .route("/usage/summary", get(usage_summary))
        .route("/usage/re-extract", post(re_extract))
        .route(
            "/usage/mem0-config",
            get(get_mem0_config).post(put_mem0_config),
        )
        .route("/usage/mem0-relevance", post(report_mem0_relevance))
        .route("/usage/token-telemetry", post(report_token_telemetry))
}

/// Body posted by the `vibe_kanban_mcp` process (a separate process from
/// this server) after each `memory_search` call, once per call — see
/// `crates/mcp/src/task_server/tools/mem0.rs`.
#[derive(Debug, Deserialize)]
struct ReportMem0RelevanceBody {
    /// Highest-ranked hit's score, or `None` when the call returned zero
    /// hits.
    top_score: Option<f64>,
}

/// Best-effort sink: always returns success, even if nothing was recorded,
/// so a malformed/late report from the MCP process never surfaces as an
/// error to it — this is an observability aid, not a critical write path.
async fn report_mem0_relevance(
    State(deployment): State<DeploymentImpl>,
    Json(body): Json<ReportMem0RelevanceBody>,
) -> ResponseJson<ApiResponse<()>> {
    deployment.mem0_relevance_service().record(body.top_score);
    ResponseJson(ApiResponse::success(()))
}

/// Body posted by the frontend (or any caller) when a session accumulates
/// token usage — typically on execution completion or periodic flush.
#[derive(Debug, Deserialize)]
struct ReportTokenTelemetryBody {
    agent: String,
    #[serde(default)]
    input_tokens: u32,
    #[serde(default)]
    output_tokens: u32,
    #[serde(default)]
    cache_read_tokens: u32,
    #[serde(default)]
    cache_creation_tokens: u32,
}

/// Best-effort sink: always returns success — same pattern as
/// [`report_mem0_relevance`]. An observability aid, not a critical path.
async fn report_token_telemetry(
    State(deployment): State<DeploymentImpl>,
    Json(body): Json<ReportTokenTelemetryBody>,
) -> ResponseJson<ApiResponse<()>> {
    deployment.token_telemetry_service().record(
        &body.agent,
        body.input_tokens,
        body.output_tokens,
        body.cache_read_tokens,
        body.cache_creation_tokens,
    );
    ResponseJson(ApiResponse::success(()))
}

async fn get_mem0_config(
    State(deployment): State<DeploymentImpl>,
) -> ResponseJson<ApiResponse<Mem0Config>> {
    let _ = deployment;
    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
    {
        Ok(c) => c,
        Err(_) => return ResponseJson(ApiResponse::error("failed to build mem0 client")),
    };
    let url = format!("{}/api/config", mem0_url());
    match client.get(&url).send().await {
        Ok(r) if r.status().is_success() => match r.json::<Mem0Config>().await {
            Ok(cfg) => ResponseJson(ApiResponse::success(cfg)),
            Err(_) => ResponseJson(ApiResponse::error("failed to parse mem0 config")),
        },
        Ok(r) => ResponseJson(ApiResponse::error(&format!(
            "mem0 config returned status {}",
            r.status()
        ))),
        Err(e) => ResponseJson(ApiResponse::error(&format!("mem0 config failed: {e}"))),
    }
}

async fn put_mem0_config(
    State(deployment): State<DeploymentImpl>,
    Json(req): Json<UpdateMem0ConfigRequest>,
) -> ResponseJson<ApiResponse<Mem0Config>> {
    let _ = deployment;
    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
    {
        Ok(c) => c,
        Err(_) => return ResponseJson(ApiResponse::error("failed to build mem0 client")),
    };
    let url = format!("{}/api/config", mem0_url());
    let body = serde_json::json!({
        "provider": req.provider,
        "graph_enabled": req.graph_enabled,
        "providers": req.providers,
    });
    match client.post(&url).json(&body).send().await {
        Ok(r) if r.status().is_success() => match r.json::<Mem0Config>().await {
            Ok(cfg) => ResponseJson(ApiResponse::success(cfg)),
            Err(_) => ResponseJson(ApiResponse::error("failed to parse mem0 config")),
        },
        Ok(r) => ResponseJson(ApiResponse::error(&format!(
            "mem0 config update returned status {}",
            r.status()
        ))),
        Err(e) => ResponseJson(ApiResponse::error(&format!(
            "mem0 config update failed: {e}"
        ))),
    }
}

/// Fetch mem0 extraction-token usage (best-effort).
async fn fetch_mem0_tokens() -> Mem0TokenUsage {
    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
    {
        Ok(c) => c,
        Err(_) => return Mem0TokenUsage::default(),
    };
    let url = format!("{}/api/usage/tokens", mem0_url());
    let resp = match client.get(&url).send().await {
        Ok(r) if r.status().is_success() => r,
        _ => return Mem0TokenUsage::default(),
    };
    match resp.json::<Mem0TokenUsage>().await {
        Ok(t) => t,
        Err(_) => Mem0TokenUsage::default(),
    }
}

async fn usage_summary(
    State(deployment): State<DeploymentImpl>,
) -> ResponseJson<ApiResponse<UsageSummary>> {
    let pool = &deployment.db().pool;

    // Per-day, per-agent execution counts and durations (exclude still-running
    // executions from the duration sum — their completed_at is NULL).
    let activity = sqlx::query_as::<_, (String, String, i64, i64)>(
        r#"SELECT date(ep.started_at) AS day,
                  COALESCE(s.executor, 'unknown') AS agent,
                  COUNT(*) AS executions,
                  COALESCE(SUM(
                      CASE
                        WHEN ep.completed_at IS NOT NULL
                          THEN CAST((julianday(ep.completed_at) - julianday(ep.started_at)) * 86400 AS INTEGER)
                        ELSE 0
                      END
                  ), 0) AS seconds
           FROM execution_processes ep
           LEFT JOIN sessions s ON s.id = ep.session_id
           WHERE ep.started_at >= datetime('now', '-30 days')
           GROUP BY day, agent
           ORDER BY day ASC"#,
    )
    .fetch_all(pool)
    .await
    .unwrap_or_default()
    .into_iter()
    .map(|(day, agent, executions, seconds)| DailyAgentActivity {
        day,
        agent,
        executions,
        seconds,
    })
    .collect();

    // Issue created/completed per day.
    let issues = sqlx::query_as::<_, (String, i64, i64)>(
        r#"SELECT COALESCE(d.day, '') AS day,
                  COALESCE(SUM(d.created), 0) AS created,
                  COALESCE(SUM(d.completed), 0) AS completed
           FROM (
               SELECT date(created_at) AS day, 1 AS created, 0 AS completed
                 FROM issues WHERE created_at >= datetime('now', '-30 days')
               UNION ALL
               SELECT date(completed_at) AS day, 0 AS created, 1 AS completed
                 FROM issues WHERE completed_at IS NOT NULL
                               AND completed_at >= datetime('now', '-30 days')
           ) d
           GROUP BY d.day
           ORDER BY d.day ASC"#,
    )
    .fetch_all(pool)
    .await
    .unwrap_or_default()
    .into_iter()
    .map(|(day, created, completed)| DailyIssueActivity {
        day,
        created,
        completed,
    })
    .collect();

    // Per-project open/done counts (done = completed_at set).
    #[derive(FromRow)]
    struct ProjectRow {
        id: Uuid,
        name: String,
        total: i64,
        done: i64,
    }
    let projects = sqlx::query_as::<_, ProjectRow>(
        r#"SELECT p.id, p.name,
                  COUNT(i.id) AS total,
                  COALESCE(SUM(CASE WHEN i.completed_at IS NOT NULL THEN 1 ELSE 0 END), 0) AS done
           FROM projects p
           LEFT JOIN issues i ON i.project_id = p.id
           GROUP BY p.id, p.name
           ORDER BY p.name ASC"#,
    )
    .fetch_all(pool)
    .await
    .unwrap_or_default()
    .into_iter()
    .map(|row| ProjectProgress {
        project_id: row.id.to_string(),
        name: row.name,
        total: row.total,
        done: row.done,
        open: row.total - row.done,
    })
    .collect();

    // Totals across the whole 30-day window.
    let (total_executions, total_seconds) = sqlx::query_as::<_, (i64, i64)>(
        r#"SELECT COUNT(*) AS executions,
                  COALESCE(SUM(
                      CASE
                        WHEN completed_at IS NOT NULL
                          THEN CAST((julianday(completed_at) - julianday(started_at)) * 86400 AS INTEGER)
                        ELSE 0
                      END
                  ), 0) AS seconds
           FROM execution_processes
           WHERE started_at >= datetime('now', '-30 days')"#,
    )
    .fetch_one(pool)
    .await
    .unwrap_or((0, 0));

    let mem0_tokens = fetch_mem0_tokens().await;
    let mem0_relevance = deployment.mem0_relevance_service().summary();
    let token_telemetry = deployment.token_telemetry_service().summary();

    ResponseJson(ApiResponse::success(UsageSummary {
        activity,
        issues,
        projects,
        total_executions,
        total_seconds,
        mem0_tokens,
        mem0_relevance,
        token_telemetry,
    }))
}

/// Proxy to the mem0 server's `POST /api/re-extract/:user_id` — re-runs graph
/// extraction for memories stored before an extraction LLM was configured.
/// `?user_id=` selects which repository's memories to re-extract.
async fn re_extract(
    State(deployment): State<DeploymentImpl>,
    axum::extract::Query(q): axum::extract::Query<ReExtractQuery>,
) -> ResponseJson<ApiResponse<ReExtractResponse>> {
    let _ = deployment;
    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(300))
        .build()
    {
        Ok(c) => c,
        Err(_) => {
            return ResponseJson(ApiResponse::error("failed to build mem0 client"));
        }
    };
    let user_id = q.user_id.unwrap_or_else(|| "default".to_string());
    let url = format!("{}/api/re-extract/{}", mem0_url(), user_id);
    match client.post(&url).send().await {
        Ok(r) if r.status().is_success() => match r.json::<ReExtractResponse>().await {
            Ok(res) => ResponseJson(ApiResponse::success(res)),
            Err(_) => ResponseJson(ApiResponse::error(
                "failed to parse mem0 re-extract response",
            )),
        },
        Ok(r) => ResponseJson(ApiResponse::error(&format!(
            "mem0 re-extract returned status {}",
            r.status()
        ))),
        Err(e) => ResponseJson(ApiResponse::error(&format!("mem0 re-extract failed: {e}"))),
    }
}

#[derive(Debug, Deserialize)]
struct ReExtractQuery {
    user_id: Option<String>,
}
