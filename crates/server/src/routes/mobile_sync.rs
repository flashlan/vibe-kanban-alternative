//! Local context export consumed by the AuraPunk Cloud sync client.
//!
//! This endpoint is intentionally read-only. The desktop frontend adds the
//! account-scoped device token when it pushes this snapshot to Cloud; the
//! local server never trusts a browser-supplied account id for authorization.

use axum::{Router, extract::State, response::Json as ResponseJson, routing::get};
use db::models::{
    issue::Issue, issue_workspace::IssueWorkspace, project::Project, project_status::ProjectStatus,
    workspace::Workspace,
};
use deployment::Deployment;
use serde::Serialize;
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

pub fn router() -> Router<DeploymentImpl> {
    Router::new().route("/mobile/context", get(get_context))
}
