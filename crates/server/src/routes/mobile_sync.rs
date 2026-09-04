//! Context export and local dispatch endpoints consumed by the Mobile/Cloud
//! sync path. The local server never trusts a browser-supplied account id for
//! authorization and remains the only component allowed to start an executor.

use api_types::UpdateIssueRequest;
use axum::{
    Json, Router,
    extract::{State, ws::Message},
    http::HeaderMap,
    response::{IntoResponse, Json as ResponseJson, Response},
    routing::{get, patch, post},
};
use base64::Engine;
use db::models::{
    file::WorkspaceAttachment,
    issue::Issue,
    issue_workspace::IssueWorkspace,
    project::Project,
    project_repo::ProjectRepo,
    project_status::ProjectStatus,
    repo::Repo,
    requests::{CreateAndStartWorkspaceRequest, LinkedIssueInfo, WorkspaceRepoInput},
    session::{CreateSession, Session},
    workspace::Workspace,
};
use deployment::Deployment;
use executors::model_selector::PermissionPolicy;
use executors::{executors::BaseCodingAgent, profile::ExecutorConfig};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::Row;
use utils::response::ApiResponse;
use uuid::Uuid;

use crate::{
    DeploymentImpl,
    error::ApiError,
    middleware::signed_ws::{MaybeSignedWebSocket, SignedWsUpgrade},
    routes::{instance, local_kanban},
};

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
    #[serde(default)]
    pub model_id: Option<String>,
    #[serde(default)]
    pub reasoning_id: Option<String>,
    #[serde(default)]
    pub agent_id: Option<String>,
    #[serde(default)]
    pub permission_policy: Option<PermissionPolicy>,
    #[serde(default)]
    pub attachments: Vec<MobileChatAttachment>,
}

#[derive(Debug, Deserialize)]
pub struct MobileChatAttachment {
    pub file_name: String,
    pub mime_type: String,
    pub data_base64: String,
}

#[derive(Debug, Deserialize)]
pub struct MobileWorkspaceRequest {
    pub issue_id: Uuid,
    pub executor: Option<String>,
}

/// Export the current local context in the same record shape accepted by the
/// Cloud `/api/sync` endpoint. The endpoint does not mutate the local DB.
pub async fn get_context(
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<MobileContextResponse>>, ApiError> {
    let pool = &deployment.db().pool;
    let mut records = Vec::new();
    let node = instance::describe(&deployment);
    records.push(MobileSyncRecord {
        entity_type: "instance",
        entity_id: node.instance_id.clone(),
        operation: "upsert",
        payload: serde_json::to_value(node)
            .map_err(|error| ApiError::BadRequest(error.to_string()))?,
    });
    let issue_workspace_links = IssueWorkspace::list_linked_all(pool).await?;

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
            let mut payload = serde_json::to_value(&issue)
                .map_err(|error| ApiError::BadRequest(error.to_string()))?;
            if let Some(link) = issue_workspace_links
                .iter()
                .find(|link| link.issue_id == issue.id)
            {
                if let Some(object) = payload.as_object_mut() {
                    object.insert(
                        "workspace_id".to_string(),
                        serde_json::to_value(link.workspace_id)
                            .map_err(|error| ApiError::BadRequest(error.to_string()))?,
                    );
                }
            }
            records.push(MobileSyncRecord {
                entity_type: "issue",
                entity_id: issue.id.to_string(),
                operation: "upsert",
                payload,
            });
        }
    }

    // Keep the relationship that lets a mobile card open the conversation of
    // the workspace launched for that issue. The workspace and issue records
    // alone do not carry this association.
    for link in issue_workspace_links {
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

    let mut prompt = prompt.to_string();
    let mut attachment_paths = Vec::new();
    for attachment in command.attachments {
        if !attachment.mime_type.starts_with("image/") {
            return Err(ApiError::BadRequest(
                "Mobile chat attachments must be images".to_string(),
            ));
        }
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(&attachment.data_base64)
            .map_err(|_| ApiError::BadRequest("Invalid mobile image attachment".to_string()))?;
        if bytes.len() > 350_000 {
            return Err(ApiError::BadRequest(
                "Mobile image attachments must be smaller than 350 KB".to_string(),
            ));
        }
        let file = deployment
            .file()
            .store_file(&bytes, &attachment.file_name)
            .await?;
        WorkspaceAttachment::associate_many_dedup(pool, workspace.id, &[file.id]).await?;
        attachment_paths.push((file.original_name, file.file_path));
    }
    if !attachment_paths.is_empty() {
        prompt.push_str("\n\n## Images attached from AuraPunk Mobile\n");
        for (name, path) in attachment_paths {
            prompt.push_str(&format!("- Inspect `.vibe-attachments/{path}` ({name})\n"));
        }
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
    let mut executor_config = ExecutorConfig::new(executor);
    executor_config.model_id = command.model_id;
    executor_config.reasoning_id = command.reasoning_id;
    executor_config.agent_id = command.agent_id;
    executor_config.permission_policy = command.permission_policy;
    Ok(crate::routes::sessions::run_follow_up(
        &deployment,
        session,
        workspace,
        prompt,
        executor_config,
        None,
        None,
        None,
    )
    .await?
    .into_response())
}

/// Create and start the workspace for a card requested by Mobile. The browser
/// or Mobile app never receives permission to create worktrees directly: the
/// local Desktop remains the owner of repositories, sessions and executors.
pub async fn post_workspace_request(
    State(deployment): State<DeploymentImpl>,
    Json(request): Json<MobileWorkspaceRequest>,
) -> Result<Response, ApiError> {
    let pool = &deployment.db().pool;
    let issue = Issue::find_by_id(pool, request.issue_id)
        .await?
        .ok_or_else(|| ApiError::BadRequest("issue not found".to_string()))?;

    if let Some(workspace_id) = IssueWorkspace::find_latest_by_issue(pool, issue.id).await?
        && let Some(workspace) = Workspace::find_by_id(pool, workspace_id).await?
        && !workspace.archived
    {
        return Ok(
            ResponseJson(ApiResponse::<Value>::success(serde_json::json!({
                "workspace_id": workspace.id,
                "created": false,
            })))
            .into_response(),
        );
    }

    let repo_ids = ProjectRepo::list_repo_ids(pool, issue.project_id).await?;
    let repos = Repo::find_by_ids(pool, &repo_ids).await?;
    if repos.is_empty() {
        return Err(ApiError::BadRequest(
            "the issue project has no repository configured".to_string(),
        ));
    }

    let executor = request
        .executor
        .as_deref()
        .unwrap_or("CODEX")
        .parse::<BaseCodingAgent>()
        .map_err(|_| ApiError::BadRequest("unknown mobile workspace executor".to_string()))?;
    let repos = repos
        .into_iter()
        .map(|repo| WorkspaceRepoInput {
            repo_id: repo.id,
            target_branch: repo
                .default_target_branch
                .unwrap_or_else(|| "main".to_string()),
        })
        .collect();
    let prompt = match issue.description.as_deref() {
        Some(description) if !description.trim().is_empty() => {
            format!("{}\n\n{}", issue.title, description)
        }
        _ => issue.title.clone(),
    };

    let response = crate::routes::workspaces::create::create_and_start_workspace(
        State(deployment),
        Json(CreateAndStartWorkspaceRequest {
            name: Some(issue.simple_id.clone()),
            repos,
            linked_issue: Some(LinkedIssueInfo {
                remote_project_id: issue.project_id,
                issue_id: issue.id,
            }),
            executor_config: ExecutorConfig::new(executor),
            prompt,
            attachment_ids: None,
            kind: None,
        }),
    )
    .await?;

    Ok(response.into_response())
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
        .route("/mobile/workspace", post(post_workspace_request))
}

/// Direct Mobile transport over the configured Tailcat network. The bearer
/// token is generated per Desktop instance and is separate from the Cloud
/// account token. Tailcat ACLs provide network isolation; this token provides
/// an application-level second check.
pub fn tailcat_router() -> Router<DeploymentImpl> {
    Router::new()
        .route("/tailcat/health", get(tailcat_health))
        .route("/tailcat/context", get(tailcat_context))
        .route("/tailcat/chat", post(tailcat_chat))
        .route("/tailcat/workspace", post(tailcat_workspace))
        .route("/tailcat/issues/{id}", patch(tailcat_update_issue))
        .route("/tailcat/events/ws", get(tailcat_events_ws))
}

async fn tailcat_health(
    headers: HeaderMap,
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<Value>>, ApiError> {
    authorize_tailcat(&headers, &deployment)?;
    Ok(ResponseJson(ApiResponse::success(serde_json::json!({
        "service": "aurapunk-desktop",
        "transport": "tailcat",
        "instance_id": instance::describe(&deployment).instance_id,
    }))))
}

async fn tailcat_context(
    headers: HeaderMap,
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<MobileContextResponse>>, ApiError> {
    authorize_tailcat(&headers, &deployment)?;
    Ok(get_context(State(deployment)).await?)
}

async fn tailcat_chat(
    headers: HeaderMap,
    State(deployment): State<DeploymentImpl>,
    Json(command): Json<MobileChatCommand>,
) -> Result<Response, ApiError> {
    authorize_tailcat(&headers, &deployment)?;
    post_chat_command(State(deployment), Json(command)).await
}

async fn tailcat_workspace(
    headers: HeaderMap,
    State(deployment): State<DeploymentImpl>,
    Json(request): Json<MobileWorkspaceRequest>,
) -> Result<Response, ApiError> {
    authorize_tailcat(&headers, &deployment)?;
    post_workspace_request(State(deployment), Json(request)).await
}

async fn tailcat_update_issue(
    headers: HeaderMap,
    State(deployment): State<DeploymentImpl>,
    axum::extract::Path(id): axum::extract::Path<Uuid>,
    Json(request): Json<UpdateIssueRequest>,
) -> Result<Response, ApiError> {
    authorize_tailcat(&headers, &deployment)?;
    let issue = local_kanban::merge_and_update_issue(&deployment.db().pool, id, request)
        .await?
        .ok_or_else(|| ApiError::BadRequest("issue not found".into()))?;
    Ok(ResponseJson(ApiResponse::<_, Value>::success(issue)).into_response())
}

async fn tailcat_events_ws(
    ws: SignedWsUpgrade,
    headers: HeaderMap,
    State(deployment): State<DeploymentImpl>,
) -> Response {
    if let Err(error) = authorize_tailcat(&headers, &deployment) {
        return error.into_response();
    }
    ws.on_upgrade(move |mut socket: MaybeSignedWebSocket| async move {
        use futures_util::StreamExt;

        let mut events = deployment.stream_events().await;
        while let Some(event) = events.next().await {
            if event.is_err() {
                break;
            }
            if socket
                .send(Message::Text("{\"type\":\"context_changed\"}".into()))
                .await
                .is_err()
            {
                break;
            }
        }
    })
    .into_response()
}

fn authorize_tailcat(headers: &HeaderMap, deployment: &impl Deployment) -> Result<(), ApiError> {
    let expected = instance::describe(deployment).direct_token;
    let supplied = headers
        .get("authorization")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .unwrap_or_default();
    if supplied.is_empty() || supplied != expected {
        return Err(ApiError::Forbidden("invalid Tailcat instance token".into()));
    }
    Ok(())
}
