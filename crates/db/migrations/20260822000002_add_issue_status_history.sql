-- Track every issue status change so we can compute lifecycle metrics
-- (review cycles, rework, total time) without polling. The trigger fires on
-- every status change regardless of code path (drag/drop, auto-move, API, MCP)
-- and is a no-op when the status is unchanged.

CREATE TABLE issue_status_history (
    id             TEXT PRIMARY KEY NOT NULL,
    issue_id       TEXT NOT NULL REFERENCES issues(id) ON DELETE CASCADE,
    from_status_id TEXT,
    to_status_id   TEXT NOT NULL REFERENCES project_statuses(id),
    created_at     TEXT NOT NULL DEFAULT (datetime('now', 'subsec'))
);

CREATE INDEX idx_issue_status_history_issue_id
    ON issue_status_history(issue_id, created_at);

CREATE TRIGGER trg_issue_status_history_after_update
AFTER UPDATE OF status_id ON issues
FOR EACH ROW
WHEN NEW.status_id IS NOT OLD.status_id
BEGIN
    INSERT INTO issue_status_history (id, issue_id, from_status_id, to_status_id, created_at)
    VALUES (
        lower(hex(randomblob(16))),
        NEW.id,
        OLD.status_id,
        NEW.status_id,
        datetime('now', 'subsec')
    );
END;
