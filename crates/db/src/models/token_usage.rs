use sqlx::SqlitePool;
use uuid::Uuid;

/// One normalized token-usage observation emitted by an agent execution.
///
/// Records are intentionally event-shaped rather than turn-shaped. Some CLIs
/// emit cumulative usage while others emit deltas; keeping the observation
/// index lets the reporting layer evolve its reconciliation rules without
/// losing the source data.
pub struct TokenUsageRecord;

impl TokenUsageRecord {
    #[allow(clippy::too_many_arguments)]
    pub async fn upsert(
        pool: &SqlitePool,
        execution_process_id: Uuid,
        session_id: Uuid,
        workspace_id: Uuid,
        issue_id: Option<Uuid>,
        entry_index: i64,
        agent: &str,
        provider: Option<&str>,
        model: Option<&str>,
        total_tokens: i64,
        model_context_window: i64,
        input_tokens: i64,
        output_tokens: i64,
        cache_read_tokens: i64,
        cache_creation_tokens: i64,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"INSERT INTO token_usage_records (
                    id, execution_process_id, session_id, workspace_id, issue_id,
                    entry_index, agent, provider, model, total_tokens,
                    model_context_window, input_tokens, output_tokens,
                    cache_read_tokens, cache_creation_tokens
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                ON CONFLICT(execution_process_id, entry_index) DO UPDATE SET
                    issue_id = excluded.issue_id,
                    agent = excluded.agent,
                    provider = excluded.provider,
                    model = excluded.model,
                    total_tokens = excluded.total_tokens,
                    model_context_window = excluded.model_context_window,
                    input_tokens = excluded.input_tokens,
                    output_tokens = excluded.output_tokens,
                    cache_read_tokens = excluded.cache_read_tokens,
                    cache_creation_tokens = excluded.cache_creation_tokens,
                    observed_at = datetime('now', 'subsec')"#,
        )
        .bind(Uuid::new_v4())
        .bind(execution_process_id)
        .bind(session_id)
        .bind(workspace_id)
        .bind(issue_id)
        .bind(entry_index)
        .bind(agent)
        .bind(provider)
        .bind(model)
        .bind(total_tokens)
        .bind(model_context_window)
        .bind(input_tokens)
        .bind(output_tokens)
        .bind(cache_read_tokens)
        .bind(cache_creation_tokens)
        .execute(pool)
        .await
        .map(|_| ())
    }
}
