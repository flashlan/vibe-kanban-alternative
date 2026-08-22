-- Add archive support to issues: an `archived` flag plus the timestamp it was
-- archived at. Archived issues are hidden from the active board (the board
-- reads through `list_by_project`, which now filters them out) but survive in
-- the DB so they can be recovered or permanently deleted from an archive view.
ALTER TABLE issues ADD COLUMN archived INTEGER NOT NULL DEFAULT 0;
ALTER TABLE issues ADD COLUMN archived_at TEXT;

CREATE INDEX IF NOT EXISTS idx_issues_archived ON issues(project_id, archived);
