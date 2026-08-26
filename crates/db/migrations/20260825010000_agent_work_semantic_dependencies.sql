-- Semantic coordination metadata. Agents may declare symbols/modules they
-- depend on, allowing the Integration Guard to flag contract-level overlap
-- even when the changed files are different.
ALTER TABLE agent_work_declarations
ADD COLUMN dependencies_json TEXT NOT NULL DEFAULT '[]';
