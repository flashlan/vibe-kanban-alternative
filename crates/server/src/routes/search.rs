use axum::{
    Router,
    extract::{Json, Query, State},
    response::Json as ResponseJson,
    routing::{get, post},
};
use chrono::{DateTime, Utc};
use db::models::repo::{Repo, SearchResult};
use deployment::Deployment;
use serde::{Deserialize, Serialize};
use services::services::file_search::{SearchMode, SearchQuery};
use sqlx::Row;
use utils::response::ApiResponse;
use uuid::Uuid;

use crate::{DeploymentImpl, error::ApiError};

#[derive(Debug, Deserialize)]
pub struct MultiRepoSearchQuery {
    pub q: String,
    #[serde(default)]
    pub mode: SearchMode,
    pub repo_ids: String,
}

#[derive(Debug, Deserialize)]
pub struct AgentSearchRequest {
    pub q: String,
    pub issue_id: Option<Uuid>,
    pub workspace_id: Option<Uuid>,
    #[serde(default = "default_agent_search_limit")]
    pub limit: i64,
}

fn default_agent_search_limit() -> i64 {
    5
}

#[derive(Debug, Serialize)]
pub struct AgentSearchResult {
    pub source: &'static str,
    pub execution_id: String,
    pub workspace_id: String,
    pub issue_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub prompt: Option<String>,
    pub summary: Option<String>,
}

pub async fn search_agent_history(
    State(deployment): State<DeploymentImpl>,
    Json(request): Json<AgentSearchRequest>,
) -> Result<ResponseJson<ApiResponse<Vec<AgentSearchResult>>>, ApiError> {
    let query = request.q.trim();
    if query.is_empty() {
        return Err(ApiError::BadRequest("q is required".to_string()));
    }

    let limit = request.limit.clamp(1, 10);
    let rows = sqlx::query(
        r#"SELECT cat.prompt, cat.summary, cat.created_at,
                  ep.id AS execution_id, s.workspace_id, iw.issue_id
           FROM coding_agent_turns cat
           JOIN execution_processes ep ON ep.id = cat.execution_process_id
           JOIN sessions s ON s.id = ep.session_id
           LEFT JOIN issue_workspaces iw ON iw.workspace_id = s.workspace_id
           WHERE ep.run_reason = 'codingagent'
             AND ep.dropped = FALSE
             AND (?1 IS NULL OR iw.issue_id = ?1)
             AND (?2 IS NULL OR s.workspace_id = ?2)
             AND (lower(coalesce(cat.prompt, '')) LIKE '%' || lower(?3) || '%'
               OR lower(coalesce(cat.summary, '')) LIKE '%' || lower(?3) || '%')
           ORDER BY cat.created_at DESC
           LIMIT ?4"#,
    )
    .bind(request.issue_id)
    .bind(request.workspace_id)
    .bind(query)
    .bind(limit)
    .fetch_all(&deployment.db().pool)
    .await?;

    let results = rows
        .into_iter()
        .map(|row| {
            let execution_id = Uuid::from_slice(&row.get::<Vec<u8>, _>("execution_id"))
                .map(|id| id.to_string())
                .unwrap_or_default();
            let workspace_id = Uuid::from_slice(&row.get::<Vec<u8>, _>("workspace_id"))
                .map(|id| id.to_string())
                .unwrap_or_default();
            let issue_id = row
                .try_get::<Option<Vec<u8>>, _>("issue_id")
                .ok()
                .flatten()
                .and_then(|id| Uuid::from_slice(&id).ok())
                .map(|id| id.to_string());
            AgentSearchResult {
                source: "database",
                execution_id,
                workspace_id,
                issue_id,
                created_at: row.get("created_at"),
                prompt: row.get("prompt"),
                summary: row.get("summary"),
            }
        })
        .collect();

    Ok(ResponseJson(ApiResponse::success(results)))
}

pub async fn search_files(
    State(deployment): State<DeploymentImpl>,
    Query(query): Query<MultiRepoSearchQuery>,
) -> Result<ResponseJson<ApiResponse<Vec<SearchResult>>>, ApiError> {
    let repo_ids: Vec<Uuid> = query
        .repo_ids
        .split(',')
        .filter(|s| !s.trim().is_empty())
        .map(|s| s.trim().parse::<Uuid>())
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| ApiError::BadRequest("Invalid repo_id format".to_string()))?;

    if repo_ids.is_empty() {
        return Err(ApiError::BadRequest(
            "repo_ids parameter is required".to_string(),
        ));
    }

    if query.q.trim().is_empty() {
        return Ok(ResponseJson(ApiResponse::error(
            "Query parameter 'q' is required and cannot be empty",
        )));
    }

    let repos = Repo::find_by_ids(&deployment.db().pool, &repo_ids).await?;

    let search_query = SearchQuery {
        q: query.q,
        mode: query.mode,
    };

    let results = deployment
        .repo()
        .search_files(
            deployment.file_search_cache().as_ref(),
            &repos,
            &search_query,
        )
        .await
        .map_err(|e| {
            tracing::error!("Failed to search files: {}", e);
            ApiError::BadRequest(format!("Search failed: {}", e))
        })?;

    Ok(ResponseJson(ApiResponse::success(results)))
}

pub fn router(deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    Router::new()
        .route("/search", get(search_files))
        .route("/search/agent-history", post(search_agent_history))
        .with_state(deployment.clone())
}
