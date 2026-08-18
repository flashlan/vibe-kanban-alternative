use std::{
    collections::HashSet,
    sync::{Arc, LazyLock, Mutex},
};

use db::{DBService, models::workspace_repo::WorkspaceRepo};
use futures::StreamExt;
use regex::Regex;
use serde::Deserialize;
use utils::msg_store::MsgStore;
use uuid::Uuid;

const MEM0_DEFAULT_URL: &str = "http://localhost:8000";

/// Case-insensitive marker for a fact the coding agent wants persisted into
/// the project's mem0 memory: `VK-MEMORY: <self-contained fact>`. The agent is
/// told (via the recall block injected at workspace start) to emit these lines
/// for anything worth remembering across sessions.
static MEMORY_MARKER_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)VK-MEMORY:\s*(.+)").expect("valid regex"));

/// Idempotency guard keyed by execution_process_id (mirrors pipeline_stage).
static TRACKED_EXECUTIONS: LazyLock<Mutex<HashSet<Uuid>>> =
    LazyLock::new(|| Mutex::new(HashSet::new()));

#[derive(Debug, Deserialize)]
struct Mem0MemoriesResponse {
    count: usize,
    memories: Vec<Mem0Memory>,
}

#[derive(Debug, Deserialize)]
struct Mem0Memory {
    #[allow(dead_code)]
    id: String,
    payload: Option<Mem0Payload>,
}

#[derive(Debug, Deserialize)]
struct Mem0Payload {
    content: Option<String>,
}

/// Fetch relevant memories from the mem0 server for a given user_id.
/// Returns a formatted string to prepend to the workspace prompt, or None
/// if mem0 is unreachable or has no memories.
pub async fn recall_memories(user_id: &str) -> Option<String> {
    let url = format!("{}/api/memories/{}", MEM0_DEFAULT_URL, user_id);

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .ok()?;

    let resp = client.get(&url).send().await.ok()?;
    if !resp.status().is_success() {
        return None;
    }

    let data: Mem0MemoriesResponse = resp.json().await.ok()?;
    if data.count == 0 || data.memories.is_empty() {
        return None;
    }

    // Deterministic ordering: sort by content so the rendered block is
    // byte-identical across calls/sessions. This keeps the injected block a
    // stable prefix (prompt/KV-cache friendly) — a non-deterministic order
    // would break the shared-prefix cache between Terminal A and Terminal B.
    let mut contents: Vec<String> = data
        .memories
        .into_iter()
        .filter_map(|m| m.payload?.content)
        .collect();
    contents.sort();
    contents.dedup();

    let mut lines = Vec::new();
    lines.push("## Relevant memories from previous sessions".to_string());
    lines.push(String::new());
    for content in contents {
        lines.push(format!("- {}", content));
    }
    lines.push(String::new());
    lines.push(
        "Use these memories as context. If you learn a new durable fact that the \
         project should remember across sessions, emit a line exactly like \
         `VK-MEMORY: <the fact>` so it is persisted to the shared project memory."
            .to_string(),
    );

    Some(lines.join("\n"))
}

/// Save a memory to the mem0 server.
pub async fn save_memory(user_id: &str, content: &str) -> bool {
    let url = format!("{}/api/memories", MEM0_DEFAULT_URL);

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .unwrap_or_default();

    let body = serde_json::json!({
        "content": content,
        "user_id": user_id,
    });

    client
        .post(&url)
        .json(&body)
        .send()
        .await
        .map(|r| r.status().is_success())
        .unwrap_or(false)
}

/// Extract the first `VK-MEMORY: <fact>` occurrence from a line, if any.
pub fn parse_memory_marker(line: &str) -> Option<String> {
    MEMORY_MARKER_RE
        .captures(line)
        .and_then(|caps| caps.get(1))
        .map(|m| m.as_str().trim().to_string())
        .filter(|s| !s.is_empty())
}

/// Watch an execution's raw log stream for `VK-MEMORY:` markers and persist
/// each fact to the project's mem0 memory. `user_id` is the repo slug (shared
/// across CLIs), resolved from the workspace's repos. No-op if mem0 is
/// unreachable (memory is best-effort, never blocks work).
///
/// Mirrors `pipeline_stage::spawn_pipeline_stage_tracker`: reads complete
/// lines from `MsgStore::stdout_lines_stream()` (stops at `LogMsg::Finished`),
/// and is idempotent per execution_process_id.
pub fn spawn_memory_tracker(
    store: Arc<MsgStore>,
    workspace_id: Uuid,
    execution_process_id: Uuid,
    db: DBService,
) {
    {
        let mut tracked = TRACKED_EXECUTIONS.lock().unwrap();
        if !tracked.insert(execution_process_id) {
            return;
        }
    }

    tokio::spawn(async move {
        let user_id = WorkspaceRepo::find_repos_for_workspace(&db.pool, workspace_id)
            .await
            .ok()
            .and_then(|repos| repos.first().map(|r| r.name.clone()))
            .unwrap_or_else(|| workspace_id.to_string());

        let mut lines = store.stdout_lines_stream();
        while let Some(line) = lines.next().await {
            let Ok(line) = line else { continue };
            if let Some(fact) = parse_memory_marker(&line) {
                let ok = save_memory(&user_id, &fact).await;
                if ok {
                    tracing::info!(
                        execution_process_id = %execution_process_id,
                        user_id = %user_id,
                        "Saved VK-MEMORY fact to mem0"
                    );
                } else {
                    tracing::warn!("Failed to save VK-MEMORY fact to mem0 (user_id={user_id})");
                }
            }
        }

        TRACKED_EXECUTIONS
            .lock()
            .unwrap()
            .remove(&execution_process_id);
    });
}

#[cfg(test)]
mod tests {
    use super::parse_memory_marker;

    #[test]
    fn parses_memory_marker() {
        assert_eq!(
            parse_memory_marker("VK-MEMORY: the merge uses a 3-arg compare-and-swap"),
            Some("the merge uses a 3-arg compare-and-swap".to_string())
        );
        assert_eq!(
            parse_memory_marker("vk-memory: lower-case marker"),
            Some("lower-case marker".to_string())
        );
    }

    #[test]
    fn ignores_non_marker_lines() {
        assert_eq!(parse_memory_marker("Just a normal log line"), None);
        assert_eq!(parse_memory_marker("VK-PIPELINE-STAGE: 3"), None);
        assert_eq!(parse_memory_marker("VK-MEMORY:"), None);
        assert_eq!(parse_memory_marker("VK-MEMORY:   "), None);
    }

    #[test]
    fn memory_is_best_effort_when_empty_content() {
        // Marker with content that is only whitespace must not produce a fact.
        assert_eq!(parse_memory_marker("VK-MEMORY: \n"), None);
    }
}
