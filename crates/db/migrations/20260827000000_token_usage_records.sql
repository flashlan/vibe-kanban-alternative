-- Durable, per-execution token observations emitted by the normalized log
-- pipeline. The unique execution/index pair makes ingestion safe when a
-- headed session is re-adopted or a normalizer replays its history.
CREATE TABLE token_usage_records (
    id                   BLOB PRIMARY KEY,
    execution_process_id BLOB NOT NULL REFERENCES execution_processes(id) ON DELETE CASCADE,
    session_id           BLOB NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
    workspace_id         BLOB NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    issue_id             BLOB,
    entry_index          INTEGER NOT NULL,
    agent                TEXT NOT NULL,
    provider             TEXT,
    model                TEXT,
    total_tokens         INTEGER NOT NULL DEFAULT 0,
    model_context_window INTEGER NOT NULL DEFAULT 0,
    input_tokens         INTEGER NOT NULL DEFAULT 0,
    output_tokens        INTEGER NOT NULL DEFAULT 0,
    cache_read_tokens    INTEGER NOT NULL DEFAULT 0,
    cache_creation_tokens INTEGER NOT NULL DEFAULT 0,
    observed_at          TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    UNIQUE (execution_process_id, entry_index)
);

CREATE INDEX idx_token_usage_records_issue
    ON token_usage_records (issue_id, observed_at);
CREATE INDEX idx_token_usage_records_agent_model
    ON token_usage_records (agent, provider, model, observed_at);
