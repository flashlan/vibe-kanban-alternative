//! Explicit, preview-first migration between the two supported memory APIs.
//!
//! Migration is deliberately kept outside the active-memory configuration:
//! source and destination credentials are submitted for one operation and are
//! never persisted. This avoids silently replacing the configured adapter or
//! storing a second set of secrets just because an import was requested.

use std::collections::HashSet;

use axum::{Json, Router, extract::State, routing::post};
use reqwest::{Client, RequestBuilder};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use utils::response::ApiResponse;

use crate::DeploymentImpl;

const PLATFORM_PAGE_SIZE: u32 = 200;
const MAX_MEMORIES: usize = 50_000;

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum MigrationAdapter {
    Mem0Vk,
    Mem0Platform,
}

impl MigrationAdapter {
    fn from_str(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "mem0_vk" | "mem0-vk" | "self_hosted" | "local" => Some(Self::Mem0Vk),
            "mem0_platform" | "mem0-platform" | "platform" | "cloud" => Some(Self::Mem0Platform),
            _ => None,
        }
    }
}

#[derive(Debug, Deserialize)]
struct MigrationEndpoint {
    adapter: String,
    url: String,
    /// Used only for this request. Never included in the response.
    #[serde(default)]
    api_key: Option<String>,
}

#[derive(Debug, Deserialize)]
struct MigrationRequest {
    source: MigrationEndpoint,
    destination: MigrationEndpoint,
    user_id: String,
    /// `preview` is the safe default. `execute` requires `confirm=true`.
    #[serde(default = "default_mode")]
    mode: String,
    #[serde(default)]
    confirm: bool,
}

fn default_mode() -> String {
    "preview".to_string()
}

#[derive(Debug, Clone)]
struct MigrationMemory {
    id: Option<String>,
    content: String,
    commit_sha: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct MemoryMigrationResult {
    pub mode: String,
    pub user_id: String,
    pub source_count: usize,
    pub destination_existing: usize,
    pub would_migrate: usize,
    pub queued: usize,
    pub skipped_duplicates: usize,
    pub failed: Vec<String>,
    pub warnings: Vec<String>,
}

pub fn router() -> Router<DeploymentImpl> {
    Router::new().route("/usage/memory-migration", post(migrate_memories))
}

async fn migrate_memories(
    State(_deployment): State<DeploymentImpl>,
    Json(request): Json<MigrationRequest>,
) -> Json<ApiResponse<MemoryMigrationResult>> {
    match run_migration(request).await {
        Ok(result) => Json(ApiResponse::success(result)),
        Err(error) => Json(ApiResponse::error(&error)),
    }
}

async fn run_migration(request: MigrationRequest) -> Result<MemoryMigrationResult, String> {
    let source = MigrationAdapter::from_str(&request.source.adapter)
        .ok_or_else(|| "source.adapter must be mem0_vk or mem0_platform".to_string())?;
    let destination = MigrationAdapter::from_str(&request.destination.adapter)
        .ok_or_else(|| "destination.adapter must be mem0_vk or mem0_platform".to_string())?;
    let mode = request.mode.trim().to_ascii_lowercase();
    if mode != "preview" && mode != "execute" {
        return Err("mode must be preview or execute".to_string());
    }
    if mode == "execute" && !request.confirm {
        return Err("execute requires confirm=true after reviewing a preview".to_string());
    }
    validate_user_id(&request.user_id)?;

    let source_url = normalize_url(&request.source.url)?;
    let destination_url = normalize_url(&request.destination.url)?;
    if source == destination
        && source_url == destination_url
        && same_secret(&request.source.api_key, &request.destination.api_key)
    {
        return Err("source and destination must be different memory stores".to_string());
    }

    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|error| format!("failed to build migration client: {error}"))?;
    let source_memories = fetch_memories(
        &client,
        source,
        &source_url,
        request.source.api_key.as_deref(),
        &request.user_id,
    )
    .await?;
    let destination_memories = fetch_memories(
        &client,
        destination,
        &destination_url,
        request.destination.api_key.as_deref(),
        &request.user_id,
    )
    .await?;

    let mut existing = destination_memories
        .iter()
        .map(|memory| normalize_content(&memory.content))
        .collect::<HashSet<_>>();
    let mut result = MemoryMigrationResult {
        mode: mode.clone(),
        user_id: request.user_id.clone(),
        source_count: source_memories.len(),
        destination_existing: destination_memories.len(),
        would_migrate: 0,
        queued: 0,
        skipped_duplicates: 0,
        failed: Vec::new(),
        warnings: Vec::new(),
    };

    if source == MigrationAdapter::Mem0Vk && destination == MigrationAdapter::Mem0Platform {
        result.warnings.push(
            "Platform processes imported memories asynchronously; wait for its events before switching adapters."
                .to_string(),
        );
    }
    if source == MigrationAdapter::Mem0Platform && destination == MigrationAdapter::Mem0Vk {
        result.warnings.push(
            "Imported Platform memories are re-extracted by mem0_vk; graph fields and commit provenance are recreated only when available in the source metadata."
                .to_string(),
        );
    }

    for memory in source_memories {
        let normalized = normalize_content(&memory.content);
        if normalized.is_empty() || !existing.insert(normalized) {
            result.skipped_duplicates += 1;
            continue;
        }
        result.would_migrate += 1;
        if mode == "preview" {
            continue;
        }

        match enqueue_memory(
            &client,
            destination,
            &destination_url,
            request.destination.api_key.as_deref(),
            &request.user_id,
            &memory,
        )
        .await
        {
            Ok(()) => result.queued += 1,
            Err(error) => result.failed.push(error),
        }
    }

    Ok(result)
}

async fn fetch_memories(
    client: &Client,
    adapter: MigrationAdapter,
    base_url: &str,
    api_key: Option<&str>,
    user_id: &str,
) -> Result<Vec<MigrationMemory>, String> {
    match adapter {
        MigrationAdapter::Mem0Vk => {
            let response = authorized(
                client.get(format!("{base_url}/api/memories/{user_id}")),
                adapter,
                api_key,
            )
            .send()
            .await
            .map_err(|error| format!("failed to read mem0_vk memories: {error}"))?;
            let body = response_json(response, "reading mem0_vk memories").await?;
            let memories = body
                .get("memories")
                .and_then(Value::as_array)
                .ok_or_else(|| "mem0_vk list response did not contain memories[]".to_string())?;
            Ok(memories
                .iter()
                .filter_map(parse_local_memory)
                .take(MAX_MEMORIES)
                .collect())
        }
        MigrationAdapter::Mem0Platform => {
            let mut page = 1;
            let mut all = Vec::new();
            loop {
                let url =
                    format!("{base_url}/v3/memories/?page={page}&page_size={PLATFORM_PAGE_SIZE}");
                let response = authorized(
                    client
                        .post(url)
                        .json(&serde_json::json!({"filters": {"user_id": user_id}})),
                    adapter,
                    api_key,
                )
                .send()
                .await
                .map_err(|error| format!("failed to read Mem0 Platform memories: {error}"))?;
                let body = response_json(response, "reading Mem0 Platform memories").await?;
                let results = body
                    .get("results")
                    .and_then(Value::as_array)
                    .ok_or_else(|| {
                        "Mem0 Platform list response did not contain results[]".to_string()
                    })?;
                all.extend(results.iter().filter_map(parse_platform_memory));
                if results.len() < PLATFORM_PAGE_SIZE as usize || all.len() >= MAX_MEMORIES {
                    break;
                }
                page += 1;
            }
            all.truncate(MAX_MEMORIES);
            Ok(all)
        }
    }
}

fn parse_local_memory(value: &Value) -> Option<MigrationMemory> {
    let payload = value.get("payload").unwrap_or(value);
    let content = payload.get("content").and_then(Value::as_str)?.trim();
    if content.is_empty() {
        return None;
    }
    Some(MigrationMemory {
        id: value.get("id").and_then(Value::as_str).map(str::to_string),
        content: content.to_string(),
        commit_sha: payload
            .get("commit_sha")
            .and_then(Value::as_str)
            .map(str::to_string),
    })
}

fn parse_platform_memory(value: &Value) -> Option<MigrationMemory> {
    let content = value.get("memory").and_then(Value::as_str)?.trim();
    if content.is_empty() {
        return None;
    }
    let metadata = value
        .get("metadata")
        .cloned()
        .unwrap_or(Value::Object(Default::default()));
    Some(MigrationMemory {
        id: value.get("id").and_then(Value::as_str).map(str::to_string),
        content: content.to_string(),
        commit_sha: metadata
            .get("commit_sha")
            .and_then(Value::as_str)
            .map(str::to_string),
    })
}

async fn enqueue_memory(
    client: &Client,
    adapter: MigrationAdapter,
    base_url: &str,
    api_key: Option<&str>,
    user_id: &str,
    memory: &MigrationMemory,
) -> Result<(), String> {
    let request = match adapter {
        MigrationAdapter::Mem0Vk => {
            let mut body = serde_json::json!({"content": memory.content, "user_id": user_id});
            if let Some(commit_sha) = memory.commit_sha.as_deref() {
                body["commit_sha"] = Value::String(commit_sha.to_string());
            }
            client.post(format!("{base_url}/api/memories")).json(&body)
        }
        MigrationAdapter::Mem0Platform => {
            let mut metadata = serde_json::Map::new();
            metadata.insert(
                "source".to_string(),
                Value::String("aurapunk-memory-migration".to_string()),
            );
            if let Some(id) = memory.id.as_deref() {
                metadata.insert(
                    "source_memory_id".to_string(),
                    Value::String(id.to_string()),
                );
            }
            if let Some(commit_sha) = memory.commit_sha.as_deref() {
                metadata.insert(
                    "commit_sha".to_string(),
                    Value::String(commit_sha.to_string()),
                );
            }
            client
                .post(format!("{base_url}/v3/memories/add/"))
                .json(&serde_json::json!({
                    "messages": [{"role": "user", "content": memory.content}],
                    "user_id": user_id,
                    "metadata": metadata,
                }))
        }
    };
    let response = authorized(request, adapter, api_key)
        .send()
        .await
        .map_err(|error| format!("failed to enqueue memory: {error}"))?;
    if response.status().is_success() {
        Ok(())
    } else {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        Err(format!(
            "destination rejected memory ({status}): {}",
            truncate(&text, 180)
        ))
    }
}

fn authorized(
    request: RequestBuilder,
    adapter: MigrationAdapter,
    api_key: Option<&str>,
) -> RequestBuilder {
    match api_key.map(str::trim).filter(|key| !key.is_empty()) {
        Some(key) if adapter == MigrationAdapter::Mem0Platform => request
            .header("Authorization", format!("Token {key}"))
            .header("Accept", "application/json"),
        Some(key) => request.bearer_auth(key),
        None => request,
    }
}

async fn response_json(response: reqwest::Response, operation: &str) -> Result<Value, String> {
    let status = response.status();
    let text = response
        .text()
        .await
        .map_err(|error| format!("failed {operation}: {error}"))?;
    if !status.is_success() {
        return Err(format!(
            "{operation} returned {status}: {}",
            truncate(&text, 180)
        ));
    }
    serde_json::from_str(&text).map_err(|error| format!("invalid JSON while {operation}: {error}"))
}

fn normalize_url(url: &str) -> Result<String, String> {
    let url = url.trim().trim_end_matches('/');
    if !(url.starts_with("http://") || url.starts_with("https://")) || url.len() <= 8 {
        return Err("memory endpoint URL must start with http:// or https://".to_string());
    }
    Ok(url.to_string())
}

fn validate_user_id(user_id: &str) -> Result<(), String> {
    if user_id.is_empty()
        || user_id.len() > 200
        || !user_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
    {
        return Err(
            "user_id must be a repository slug using letters, numbers, -, _ or .".to_string(),
        );
    }
    Ok(())
}

fn normalize_content(content: &str) -> String {
    content
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_ascii_lowercase()
}

fn same_secret(left: &Option<String>, right: &Option<String>) -> bool {
    left.as_deref().unwrap_or("").trim() == right.as_deref().unwrap_or("").trim()
}

fn truncate(value: &str, max: usize) -> String {
    value.chars().take(max).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        Router,
        routing::{get, post},
    };
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };

    #[test]
    fn parses_local_qdrant_payload_and_commit_provenance() {
        let memory = parse_local_memory(&serde_json::json!({
            "id": "local-1",
            "payload": {"content": "Uses Qdrant", "commit_sha": "abc123"}
        }))
        .expect("local memory");
        assert_eq!(memory.id.as_deref(), Some("local-1"));
        assert_eq!(memory.content, "Uses Qdrant");
        assert_eq!(memory.commit_sha.as_deref(), Some("abc123"));
    }

    #[test]
    fn parses_platform_memory_and_metadata() {
        let memory = parse_platform_memory(&serde_json::json!({
            "id": "cloud-1",
            "memory": "Uses Mem0 Platform",
            "metadata": {"topic": "architecture"}
        }))
        .expect("platform memory");
        assert_eq!(memory.id.as_deref(), Some("cloud-1"));
        assert_eq!(memory.content, "Uses Mem0 Platform");
    }

    #[test]
    fn duplicate_detection_is_case_and_whitespace_insensitive() {
        let mut existing = HashSet::from([normalize_content("Uses   Qdrant")]);
        assert!(!existing.insert(normalize_content(" uses qdrant ")));
    }

    #[test]
    fn execute_requires_confirmation() {
        let request = MigrationRequest {
            source: MigrationEndpoint {
                adapter: "mem0_vk".into(),
                url: "http://a.test".into(),
                api_key: None,
            },
            destination: MigrationEndpoint {
                adapter: "mem0_platform".into(),
                url: "https://b.test".into(),
                api_key: None,
            },
            user_id: "repo".into(),
            mode: "execute".into(),
            confirm: false,
        };
        let result = tokio::runtime::Runtime::new()
            .expect("runtime")
            .block_on(run_migration(request));
        assert_eq!(
            result.unwrap_err(),
            "execute requires confirm=true after reviewing a preview"
        );
    }

    #[tokio::test]
    async fn migrates_local_memories_to_platform_and_skips_existing_content() {
        let destination_posts = Arc::new(AtomicUsize::new(0));
        let destination_posts_for_route = Arc::clone(&destination_posts);
        let source = Router::new().route(
            "/api/memories/repo",
            get(|| async {
                Json(serde_json::json!({
                    "memories": [
                        {"id": "local-1", "payload": {"content": "Uses Qdrant", "commit_sha": "abc123"}},
                        {"id": "local-2", "payload": {"content": "New durable fact"}}
                    ]
                }))
            }),
        );
        let destination = Router::new()
            .route(
                "/v3/memories/",
                post(|| async {
                    Json(serde_json::json!({
                        "count": 1,
                        "results": [{"id": "cloud-existing", "memory": "uses qdrant", "metadata": {}}]
                    }))
                }),
            )
            .route(
                "/v3/memories/add/",
                post(move |Json(body): Json<Value>| {
                    let destination_posts = Arc::clone(&destination_posts_for_route);
                    async move {
                        assert_eq!(body["user_id"], "repo");
                        assert_eq!(body["messages"][0]["content"], "New durable fact");
                        assert_eq!(body["metadata"]["source_memory_id"], "local-2");
                        destination_posts.fetch_add(1, Ordering::SeqCst);
                        Json(serde_json::json!({"status": "PENDING", "event_id": "evt-1"}))
                    }
                }),
            );
        let source_listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let source_url = format!("http://{}", source_listener.local_addr().unwrap());
        let source_task = tokio::spawn(async move {
            axum::serve(source_listener, source).await.unwrap();
        });
        let destination_listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let destination_url = format!("http://{}", destination_listener.local_addr().unwrap());
        let destination_task = tokio::spawn(async move {
            axum::serve(destination_listener, destination)
                .await
                .unwrap();
        });

        let preview = run_migration(MigrationRequest {
            source: MigrationEndpoint {
                adapter: "mem0_vk".into(),
                url: source_url.clone(),
                api_key: None,
            },
            destination: MigrationEndpoint {
                adapter: "mem0_platform".into(),
                url: destination_url.clone(),
                api_key: None,
            },
            user_id: "repo".into(),
            mode: "preview".into(),
            confirm: false,
        })
        .await
        .unwrap();
        assert_eq!(preview.source_count, 2);
        assert_eq!(preview.destination_existing, 1);
        assert_eq!(preview.would_migrate, 1);
        assert_eq!(preview.skipped_duplicates, 1);
        assert_eq!(preview.queued, 0);

        let executed = run_migration(MigrationRequest {
            source: MigrationEndpoint {
                adapter: "mem0_vk".into(),
                url: source_url,
                api_key: None,
            },
            destination: MigrationEndpoint {
                adapter: "mem0_platform".into(),
                url: destination_url,
                api_key: None,
            },
            user_id: "repo".into(),
            mode: "execute".into(),
            confirm: true,
        })
        .await
        .unwrap();
        assert_eq!(executed.queued, 1);
        assert_eq!(destination_posts.load(Ordering::SeqCst), 1);

        source_task.abort();
        destination_task.abort();
    }
}
