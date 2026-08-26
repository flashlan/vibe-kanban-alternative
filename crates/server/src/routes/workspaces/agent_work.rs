use axum::{
    Extension, Json, Router,
    extract::State,
    response::Json as ResponseJson,
    routing::{delete, get, post},
};
use db::models::agent_work::{
    AgentActivity, AgentWorkDeclaration, AgentWorkDeclarationResult, DeclareAgentWork,
};
use deployment::Deployment;
use serde::Deserialize;
use utils::response::ApiResponse;
use uuid::Uuid;

use crate::{DeploymentImpl, error::ApiError};

#[derive(Debug, Deserialize)]
pub struct DeclareAgentWorkRequest {
    pub owner_id: Uuid,
    pub execution_process_id: Option<Uuid>,
    pub agent_name: String,
    pub intent: String,
    #[serde(default)]
    pub files: Vec<String>,
    #[serde(default)]
    pub symbols: Vec<String>,
    #[serde(default)]
    pub dependencies: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct AgentWorkOwnerRequest {
    pub owner_id: Uuid,
}

pub async fn list_agent_work(
    Extension(workspace): Extension<db::models::workspace::Workspace>,
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<Vec<AgentActivity>>>, ApiError> {
    let activity =
        AgentWorkDeclaration::list_activity_for_workspace(&deployment.db().pool, workspace.id)
            .await?;
    Ok(ResponseJson(ApiResponse::success(activity)))
}

pub async fn declare_agent_work(
    Extension(workspace): Extension<db::models::workspace::Workspace>,
    State(deployment): State<DeploymentImpl>,
    Json(request): Json<DeclareAgentWorkRequest>,
) -> Result<ResponseJson<ApiResponse<AgentWorkDeclarationResult>>, ApiError> {
    validate_request(&request)?;

    let result = AgentWorkDeclaration::declare(
        &deployment.db().pool,
        &DeclareAgentWork {
            workspace_id: workspace.id,
            owner_id: request.owner_id,
            execution_process_id: request.execution_process_id,
            agent_name: request.agent_name,
            intent: request.intent,
            files: request.files,
            symbols: request.symbols,
            dependencies: request.dependencies,
        },
    )
    .await?;

    Ok(ResponseJson(ApiResponse::success(result)))
}

pub async fn heartbeat_agent_work(
    Extension(workspace): Extension<db::models::workspace::Workspace>,
    State(deployment): State<DeploymentImpl>,
    Json(request): Json<AgentWorkOwnerRequest>,
) -> Result<ResponseJson<ApiResponse<Option<AgentWorkDeclaration>>>, ApiError> {
    let declaration =
        AgentWorkDeclaration::heartbeat(&deployment.db().pool, workspace.id, request.owner_id)
            .await?;
    Ok(ResponseJson(ApiResponse::success(declaration)))
}

pub async fn release_agent_work(
    Extension(workspace): Extension<db::models::workspace::Workspace>,
    State(deployment): State<DeploymentImpl>,
    Json(request): Json<AgentWorkOwnerRequest>,
) -> Result<ResponseJson<ApiResponse<bool>>, ApiError> {
    let released =
        AgentWorkDeclaration::release(&deployment.db().pool, workspace.id, request.owner_id)
            .await?;
    Ok(ResponseJson(ApiResponse::success(released)))
}

fn validate_request(request: &DeclareAgentWorkRequest) -> Result<(), ApiError> {
    if request.agent_name.trim().is_empty() {
        return Err(ApiError::BadRequest(
            "agent_name must not be empty".to_string(),
        ));
    }
    if request.intent.trim().is_empty() {
        return Err(ApiError::BadRequest("intent must not be empty".to_string()));
    }
    if request.intent.chars().count() > 2000 {
        return Err(ApiError::BadRequest(
            "intent is limited to 2000 characters".to_string(),
        ));
    }
    if request.files.len() > 200 {
        return Err(ApiError::BadRequest(
            "files is limited to 200 entries".to_string(),
        ));
    }
    if request.symbols.len() > 200 {
        return Err(ApiError::BadRequest(
            "symbols is limited to 200 entries".to_string(),
        ));
    }
    if request.dependencies.len() > 200 {
        return Err(ApiError::BadRequest(
            "dependencies is limited to 200 entries".to_string(),
        ));
    }
    Ok(())
}

pub fn router() -> Router<DeploymentImpl> {
    Router::new()
        .route("/", get(list_agent_work).put(declare_agent_work))
        .route("/heartbeat", post(heartbeat_agent_work))
        .route("/release", delete(release_agent_work))
}
