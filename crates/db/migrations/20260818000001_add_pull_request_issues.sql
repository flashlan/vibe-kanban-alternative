CREATE TABLE IF NOT EXISTS pull_request_issues (
    id TEXT PRIMARY KEY NOT NULL,
    pull_request_id TEXT NOT NULL REFERENCES pull_requests(id) ON DELETE CASCADE,
    issue_id TEXT NOT NULL REFERENCES issues(id) ON DELETE CASCADE,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(pull_request_id, issue_id)
);

CREATE INDEX IF NOT EXISTS idx_pull_request_issues_issue ON pull_request_issues(issue_id);
CREATE INDEX IF NOT EXISTS idx_pull_request_issues_pr ON pull_request_issues(pull_request_id);
