-- Distributed Integration Guard lease. The repository is the lock key so
-- separate backend processes cannot write the same target branch concurrently.
CREATE TABLE integration_guard_locks (
    repo_id            BLOB PRIMARY KEY REFERENCES repos(id) ON DELETE CASCADE,
    owner_id           BLOB NOT NULL,
    lease_expires_at   TEXT NOT NULL,
    created_at         TEXT NOT NULL DEFAULT (datetime('now', 'subsec'))
);

CREATE INDEX idx_integration_guard_locks_expiry
    ON integration_guard_locks (lease_expires_at);
