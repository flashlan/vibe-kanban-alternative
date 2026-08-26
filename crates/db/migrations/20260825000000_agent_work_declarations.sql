-- Ephemeral coordination state for concurrent coding agents.
--
-- Declarations are leases rather than hard locks: they make intended work
-- visible and let the caller decide whether to wait, move elsewhere, or
-- continue with a shared review. Expired leases are ignored by the model.
CREATE TABLE agent_work_declarations (
    id                   BLOB PRIMARY KEY,
    workspace_id         BLOB NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    owner_id             BLOB NOT NULL,
    execution_process_id BLOB REFERENCES execution_processes(id) ON DELETE SET NULL,
    agent_name           TEXT NOT NULL,
    intent               TEXT NOT NULL,
    files_json           TEXT NOT NULL,
    symbols_json         TEXT NOT NULL,
    status               TEXT NOT NULL DEFAULT 'active'
                           CHECK (status IN ('active', 'released')),
    lease_expires_at     TEXT NOT NULL,
    created_at           TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    updated_at           TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    UNIQUE (workspace_id, owner_id)
);

CREATE INDEX idx_agent_work_declarations_workspace_active
    ON agent_work_declarations (workspace_id, status, lease_expires_at);
