//! Context export and local dispatch endpoints consumed by the Mobile/Cloud
//! sync path. The local server never trusts a browser-supplied account id for
//! authorization and remains the only component allowed to start an executor.

use api_types::UpdateIssueRequest;
use axum::{
    Json, Router,
    extract::State,
    response::{IntoResponse, Json as ResponseJson, Response},
    routing::{get, post},
};
use db::models::{
    issue::Issue,
    issue_workspace::IssueWorkspace,
    project::Project,
    project_status::ProjectStatus,
    session::{CreateSession, Session},
    workspace::Workspace,
};
use deployment::Deployment;
use executors::{executors::BaseCodingAgent, profile::ExecutorConfig};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::Row;
use utils::response::ApiResponse;
use uuid::Uuid;

use crate::{DeploymentImpl, error::ApiError};

#[derive(Debug, Clone, Serialize)]
pub struct MobileSyncRecord {
    pub entity_type: &'static str,
    pub entity_id: String,
    pub operation: &'static str,
    pub payload: Value,
}

#[derive(Debug, Serialize)]
pub struct MobileContextResponse {
    pub records: Vec<MobileSyncRecord>,
}

#[derive(Debug, Deserialize)]
pub struct MobileChatCommand {
    pub workspace_id: Uuid,
    pub prompt: String,
    pub executor: Option<String>,
}

/// Export the current local context in the same record shape accepted by the
/// Cloud `/api/sync` endpoint. The endpoint does not mutate the local DB.
pub async fn get_context(
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<MobileContextResponse>>, ApiError> {
    let pool = &deployment.db().pool;
    let mut records = Vec::new();

    for workspace in Workspace::find_all_with_status(pool, None, None).await? {
        records.push(MobileSyncRecord {
            entity_type: "workspace",
            entity_id: workspace.id.to_string(),
            operation: "upsert",
            payload: serde_json::to_value(workspace)
                .map_err(|error| ApiError::BadRequest(error.to_string()))?,
        });
    }

    for project in Project::find_all(pool).await? {
        records.push(MobileSyncRecord {
            entity_type: "project",
            entity_id: project.id.to_string(),
            operation: "upsert",
            payload: serde_json::to_value(&project)
                .map_err(|error| ApiError::BadRequest(error.to_string()))?,
        });

        for status in ProjectStatus::list_by_project(pool, project.id).await? {
            records.push(MobileSyncRecord {
                entity_type: "status",
                entity_id: status.id.to_string(),
                operation: "upsert",
                payload: serde_json::to_value(status)
                    .map_err(|error| ApiError::BadRequest(error.to_string()))?,
            });
        }

        for issue in Issue::list_by_project(pool, project.id).await? {
            records.push(MobileSyncRecord {
                entity_type: "issue",
                entity_id: issue.id.to_string(),
                operation: "upsert",
                payload: serde_json::to_value(issue)
                    .map_err(|error| ApiError::BadRequest(error.to_string()))?,
            });
        }
    }

    // Keep the relationship that lets a mobile card open the conversation of
    // the workspace launched for that issue. The workspace and issue records
    // alone do not carry this association.
    for link in IssueWorkspace::list_linked_all(pool).await? {
        records.push(MobileSyncRecord {
            entity_type: "issue_workspace",
            entity_id: format!("{}:{}", link.issue_id, link.workspace_id),
            operation: "upsert",
            payload: serde_json::json!({
                "issue_id": link.issue_id,
                "workspace_id": link.workspace_id,
                "project_id": link.project_id,
            }),
        });
    }

    let chat_rows = sqlx::query(
        r#"SELECT cat.id, s.workspace_id, cat.prompt, cat.summary, cat.seen,
                  cat.created_at, cat.updated_at
           FROM coding_agent_turns cat
           JOIN execution_processes ep ON ep.id = cat.execution_process_id
           JOIN sessions s ON s.id = ep.session_id
           WHERE ep.dropped = FALSE
           ORDER BY cat.created_at ASC"#,
    )
    .fetch_all(pool)
    .await?;

    for row in chat_rows {
        let id: Uuid = row.try_get("id")?;
        let payload = serde_json::json!({
            "id": id,
            "workspace_id": row.try_get::<Uuid, _>("workspace_id")?,
            "prompt": row.try_get::<Option<String>, _>("prompt")?,
            "summary": row.try_get::<Option<String>, _>("summary")?,
            "seen": row.try_get::<bool, _>("seen")?,
            "created_at": row.try_get::<chrono::DateTime<chrono::Utc>, _>("created_at")?,
            "updated_at": row.try_get::<chrono::DateTime<chrono::Utc>, _>("updated_at")?,
        });
        records.push(MobileSyncRecord {
            entity_type: "chat",
            entity_id: id.to_string(),
            operation: "upsert",
            payload,
        });
    }

    Ok(ResponseJson(ApiResponse::success(MobileContextResponse {
        records,
    })))
}

/// Dispatch a message received from Mobile through the local Desktop session.
/// The local server remains the only component allowed to start an executor.
pub async fn post_chat_command(
    State(deployment): State<DeploymentImpl>,
    Json(command): Json<MobileChatCommand>,
) -> Result<Response, ApiError> {
    let prompt = command.prompt.trim();
    if prompt.is_empty() || prompt.len() > 100_000 {
        return Err(ApiError::BadRequest(
            "Mobile chat prompt must contain 1 to 100000 characters".to_string(),
        ));
    }

    let pool = &deployment.db().pool;
    let workspace = Workspace::find_by_id(pool, command.workspace_id)
        .await?
        .ok_or(ApiError::Workspace(
            db::models::workspace::WorkspaceError::WorkspaceNotFound,
        ))?;

    // Status commands use the same guarded mutation path as the Desktop
    // Kanban. This keeps `/close`, `/review`, and `/in-progress` consistent
    // with Integration Guard and the normal Mem0 completion flow.
    if let Some(status_kind) = status_command(prompt) {
        if let Some((issue_id, project_id)) =
            IssueWorkspace::find_issue_and_project_by_workspace(pool, workspace.id).await?
        {
            let statuses =
                db::models::project_status::ProjectStatus::list_by_project(pool, project_id)
                    .await?;
            let target = statuses.iter().find(|status| match status_kind {
                "close" => {
                    status.is_terminal
                        || status.name.to_ascii_lowercase().contains("done")
                        || status.name.to_ascii_lowercase().contains("closed")
                        || status.name.to_ascii_lowercase().contains("conclu")
                        || status.name.to_ascii_lowercase().contains("fech")
                }
                "review" => {
                    status.name.to_ascii_lowercase().contains("review")
                        || status.name.to_ascii_lowercase().contains("revis")
                }
                "progress" => {
                    status.name.to_ascii_lowercase().contains("progress")
                        || status.name.to_ascii_lowercase().contains("progres")
                        || status.name.to_ascii_lowercase().contains("andamento")
                }
                _ => false,
            });
            let target = target.ok_or_else(|| {
                ApiError::BadRequest(format!(
                    "No status matching /{status_kind} exists in this project"
                ))
            })?;
            crate::routes::local_kanban::merge_and_update_issue(
                pool,
                issue_id,
                UpdateIssueRequest {
                    allow_unmerged_done: None,
                    status_id: Some(target.id),
                    title: None,
                    description: None,
                    priority: None,
                    start_date: None,
                    target_date: None,
                    completed_at: None,
                    sort_order: None,
                    parent_issue_id: None,
                    parent_issue_sort_order: None,
                    extension_metadata: None,
                },
            )
            .await?
            .ok_or_else(|| ApiError::BadRequest("Issue not found".to_string()))?;
            return Ok(
                ResponseJson(ApiResponse::<Value, Value>::success(serde_json::json!({
                    "status": target.name,
                    "issue_id": issue_id,
                })))
                .into_response(),
            );
        }
        return Err(ApiError::BadRequest(
            "This workspace has no linked card".to_string(),
        ));
    }

    let session =
        if let Some(session) = Session::find_latest_by_workspace_id(pool, workspace.id).await? {
            session
        } else {
            let executor = command
                .executor
                .as_deref()
                .unwrap_or("CODEX")
                .parse::<BaseCodingAgent>()
                .map_err(|_| ApiError::BadRequest("Unknown mobile chat executor".to_string()))?;
            Session::create(
                pool,
                &CreateSession {
                    executor: Some(executor.to_string()),
                    name: Some("Mobile chat".to_string()),
                },
                Uuid::new_v4(),
                workspace.id,
            )
            .await?
        };

    let executor = session
        .executor
        .as_deref()
        .unwrap_or("CODEX")
        .parse::<BaseCodingAgent>()
        .map_err(|_| {
            ApiError::BadRequest("Workspace session has an unknown executor".to_string())
        })?;
    Ok(crate::routes::sessions::run_follow_up(
        &deployment,
        session,
        workspace,
        prompt.to_string(),
        ExecutorConfig::new(executor),
        None,
        None,
        None,
    )
    .await?
    .into_response())
}

fn status_command(prompt: &str) -> Option<&'static str> {
    match prompt
        .split_whitespace()
        .next()
        .unwrap_or_default()
        .to_ascii_lowercase()
        .as_str()
    {
        "/close" | "/closed" | "/done" | "/complete" | "/fechar" | "/concluir" => Some("close"),
        "/review" | "/revisao" | "/revisão" => Some("review"),
        "/progress" | "/in-progress" | "/in_progress" | "/andamento" => Some("progress"),
        _ => None,
    }
}

pub fn router() -> Router<DeploymentImpl> {
    Router::new()
        .route("/mobile/context", get(get_context))
        .route("/mobile/chat", post(post_chat_command))
}
