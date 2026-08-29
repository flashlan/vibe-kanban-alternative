//! Runtime launch mode exposed to the frontend.
//!
//! The local build remains the default. A launcher may set
//! `VIBE_KANBAN_MODE=cloud` to opt into cloud-oriented UI affordances without
//! coupling the frontend bundle to a build-time environment variable.

use axum::{Json, Router, routing::get};
use serde::Serialize;
use utils::response::ApiResponse;

use crate::DeploymentImpl;

#[derive(Debug, Serialize)]
pub struct AppModeResponse {
    pub mode: &'static str,
    pub cloud: bool,
}

pub fn router() -> Router<DeploymentImpl> {
    Router::new().route("/app-mode", get(app_mode))
}

async fn app_mode() -> Json<ApiResponse<AppModeResponse>> {
    let cloud = std::env::var("VIBE_KANBAN_MODE")
        .map(|value| value.trim().eq_ignore_ascii_case("cloud"))
        .unwrap_or(false);

    Json(ApiResponse::success(AppModeResponse {
        mode: if cloud { "cloud" } else { "local" },
        cloud,
    }))
}
