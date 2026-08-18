use std::{
    collections::HashSet,
    sync::{Arc, LazyLock, Mutex},
};

use futures::StreamExt;
use regex::Regex;
use utils::msg_store::MsgStore;
use uuid::Uuid;

use super::notification::NotificationService;

/// Marker a coding agent emits when it reaches the `review-manual` pipeline
/// stage and needs the operator to review the result before continuing:
/// `VK-REVIEW-REQUEST: <what to review>`. On detection the backend plays the
/// configured notification sound (if enabled) so the user is alerted even when
/// away from the board.
static REVIEW_REQUEST_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)VK-REVIEW-REQUEST:\s*(.+)").expect("valid regex"));

/// Idempotency guard keyed by execution_process_id.
static TRACKED_EXECUTIONS: LazyLock<Mutex<HashSet<Uuid>>> =
    LazyLock::new(|| Mutex::new(HashSet::new()));

/// Extract the message after a `VK-REVIEW-REQUEST:` marker, if any.
pub fn parse_review_request(line: &str) -> Option<String> {
    REVIEW_REQUEST_RE
        .captures(line)
        .and_then(|caps| caps.get(1))
        .map(|m| m.as_str().trim().to_string())
        .filter(|s| !s.is_empty())
}

/// Watch an execution's raw log stream for `VK-REVIEW-REQUEST:` markers and
/// alert the operator (sound + notification) each time one appears. The agent
/// stops and waits after emitting the marker, so this is the "manual review"
/// alarm. Best-effort: a failed notification never blocks work.
pub fn spawn_review_request_tracker(
    store: Arc<MsgStore>,
    execution_process_id: Uuid,
    notification_service: NotificationService,
) {
    {
        let mut tracked = TRACKED_EXECUTIONS.lock().unwrap();
        if !tracked.insert(execution_process_id) {
            return;
        }
    }

    tokio::spawn(async move {
        let mut lines = store.stdout_lines_stream();
        while let Some(line) = lines.next().await {
            let Ok(line) = line else { continue };
            if let Some(message) = parse_review_request(&line) {
                tracing::info!(
                    execution_process_id = %execution_process_id,
                    "Manual review requested: {message}"
                );
                notification_service
                    .notify(
                        "Manual Review Required",
                        &format!("A workspace is waiting for your review:\n{message}"),
                        None,
                    )
                    .await;
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
    use super::parse_review_request;

    #[test]
    fn parses_review_request_marker() {
        assert_eq!(
            parse_review_request("VK-REVIEW-REQUEST: please check the merge diff"),
            Some("please check the merge diff".to_string())
        );
        assert_eq!(
            parse_review_request("vk-review-request: lower-case"),
            Some("lower-case".to_string())
        );
    }

    #[test]
    fn ignores_non_review_lines() {
        assert_eq!(parse_review_request("normal log line"), None);
        assert_eq!(parse_review_request("VK-MEMORY: a fact"), None);
        assert_eq!(parse_review_request("VK-REVIEW-REQUEST:"), None);
        assert_eq!(parse_review_request("VK-REVIEW-REQUEST:   "), None);
    }
}
