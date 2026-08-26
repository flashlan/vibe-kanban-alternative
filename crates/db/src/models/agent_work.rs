use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};
use ts_rs::TS;
use uuid::Uuid;

/// A declaration is intentionally short-lived. If an agent disappears
/// without releasing its work, the panel should eventually stop presenting it
/// as active and another agent should be able to continue.
pub const AGENT_WORK_LEASE_SECONDS: i64 = 10 * 60;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct AgentWorkDeclaration {
    pub id: Uuid,
    pub workspace_id: Uuid,
    pub owner_id: Uuid,
    pub execution_process_id: Option<Uuid>,
    pub agent_name: String,
    pub intent: String,
    pub files: Vec<String>,
    pub symbols: Vec<String>,
    pub dependencies: Vec<String>,
    pub lease_expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct AgentWorkConflict {
    pub workspace_id: Uuid,
    pub owner_id: Uuid,
    pub agent_name: String,
    pub intent: String,
    pub files: Vec<String>,
    pub symbols: Vec<String>,
    pub dependencies: Vec<String>,
    pub conflict_type: String,
    pub lease_expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct AgentWorkDeclarationResult {
    pub declaration: AgentWorkDeclaration,
    pub conflicts: Vec<AgentWorkConflict>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DeclareAgentWork {
    pub workspace_id: Uuid,
    pub owner_id: Uuid,
    pub execution_process_id: Option<Uuid>,
    pub agent_name: String,
    pub intent: String,
    pub files: Vec<String>,
    pub symbols: Vec<String>,
    pub dependencies: Vec<String>,
}

#[derive(Debug, Clone, FromRow)]
struct AgentWorkRow {
    id: Uuid,
    workspace_id: Uuid,
    owner_id: Uuid,
    execution_process_id: Option<Uuid>,
    agent_name: String,
    intent: String,
    files_json: String,
    symbols_json: String,
    dependencies_json: String,
    lease_expires_at: DateTime<Utc>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl AgentWorkRow {
    fn into_declaration(self) -> AgentWorkDeclaration {
        AgentWorkDeclaration {
            id: self.id,
            workspace_id: self.workspace_id,
            owner_id: self.owner_id,
            execution_process_id: self.execution_process_id,
            agent_name: self.agent_name,
            intent: self.intent,
            files: serde_json::from_str(&self.files_json).unwrap_or_default(),
            symbols: serde_json::from_str(&self.symbols_json).unwrap_or_default(),
            dependencies: serde_json::from_str(&self.dependencies_json).unwrap_or_default(),
            lease_expires_at: self.lease_expires_at,
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }
}

impl AgentWorkDeclaration {
    pub async fn declare(
        pool: &SqlitePool,
        input: &DeclareAgentWork,
    ) -> Result<AgentWorkDeclarationResult, sqlx::Error> {
        let now = Utc::now();
        let lease_expires_at = now + Duration::seconds(AGENT_WORK_LEASE_SECONDS);
        let agent_name = input.agent_name.trim();
        let intent = input.intent.trim();

        if agent_name.is_empty() || intent.is_empty() {
            return Err(sqlx::Error::Protocol(
                "agent_name and intent must not be empty".to_string(),
            ));
        }

        Self::remove_expired(pool, now).await?;
        let existing = Self::list_active(pool, input.workspace_id).await?;
        let conflicts = existing
            .iter()
            .filter(|other| other.owner_id != input.owner_id)
            .filter_map(|other| Self::conflict(other, input))
            .collect();

        let files_json = serde_json::to_string(&input.files)
            .map_err(|e| sqlx::Error::Protocol(e.to_string()))?;
        let symbols_json = serde_json::to_string(&input.symbols)
            .map_err(|e| sqlx::Error::Protocol(e.to_string()))?;
        let dependencies_json = serde_json::to_string(&input.dependencies)
            .map_err(|e| sqlx::Error::Protocol(e.to_string()))?;

        sqlx::query(
            r#"INSERT INTO agent_work_declarations (
                    id, workspace_id, owner_id, execution_process_id,
                    agent_name, intent, files_json, symbols_json, dependencies_json,
                    status, lease_expires_at, created_at, updated_at
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, 'active', ?, ?, ?)
                ON CONFLICT (workspace_id, owner_id) DO UPDATE SET
                    execution_process_id = excluded.execution_process_id,
                    agent_name = excluded.agent_name,
                    intent = excluded.intent,
                    files_json = excluded.files_json,
                    symbols_json = excluded.symbols_json,
                    dependencies_json = excluded.dependencies_json,
                    status = 'active',
                    lease_expires_at = excluded.lease_expires_at,
                    updated_at = excluded.updated_at"#,
        )
        .bind(Uuid::new_v4())
        .bind(input.workspace_id)
        .bind(input.owner_id)
        .bind(input.execution_process_id)
        .bind(agent_name)
        .bind(intent)
        .bind(files_json)
        .bind(symbols_json)
        .bind(dependencies_json)
        .bind(lease_expires_at)
        .bind(now)
        .bind(now)
        .execute(pool)
        .await?;

        let declaration = sqlx::query_as::<_, AgentWorkRow>(
            "SELECT id, workspace_id, owner_id, execution_process_id, agent_name, intent, files_json, symbols_json, dependencies_json, lease_expires_at, created_at, updated_at FROM agent_work_declarations WHERE workspace_id = ? AND owner_id = ?",
        )
        .bind(input.workspace_id)
        .bind(input.owner_id)
        .fetch_one(pool)
        .await?
        .into_declaration();

        Ok(AgentWorkDeclarationResult {
            declaration,
            conflicts,
        })
    }

    pub async fn heartbeat(
        pool: &SqlitePool,
        workspace_id: Uuid,
        owner_id: Uuid,
    ) -> Result<Option<Self>, sqlx::Error> {
        let now = Utc::now();
        let lease_expires_at = now + Duration::seconds(AGENT_WORK_LEASE_SECONDS);
        Self::remove_expired(pool, now).await?;
        sqlx::query(
            "UPDATE agent_work_declarations SET lease_expires_at = ?, updated_at = ? WHERE workspace_id = ? AND owner_id = ? AND status = 'active'",
        )
        .bind(lease_expires_at)
        .bind(now)
        .bind(workspace_id)
        .bind(owner_id)
        .execute(pool)
        .await?;

        sqlx::query_as::<_, AgentWorkRow>(
            "SELECT id, workspace_id, owner_id, execution_process_id, agent_name, intent, files_json, symbols_json, dependencies_json, lease_expires_at, created_at, updated_at FROM agent_work_declarations WHERE workspace_id = ? AND owner_id = ? AND status = 'active'",
        )
        .bind(workspace_id)
        .bind(owner_id)
        .fetch_optional(pool)
        .await
        .map(|row| row.map(AgentWorkRow::into_declaration))
    }

    pub async fn release(
        pool: &SqlitePool,
        workspace_id: Uuid,
        owner_id: Uuid,
    ) -> Result<bool, sqlx::Error> {
        let result = sqlx::query(
            "UPDATE agent_work_declarations SET status = 'released', updated_at = ?, lease_expires_at = ? WHERE workspace_id = ? AND owner_id = ? AND status = 'active'",
        )
        .bind(Utc::now())
        .bind(Utc::now())
        .bind(workspace_id)
        .bind(owner_id)
        .execute(pool)
        .await?;
        Ok(result.rows_affected() > 0)
    }

    pub async fn list_active(
        pool: &SqlitePool,
        workspace_id: Uuid,
    ) -> Result<Vec<AgentWorkDeclaration>, sqlx::Error> {
        let now = Utc::now();
        Self::remove_expired(pool, now).await?;
        sqlx::query_as::<_, AgentWorkRow>(
            "SELECT id, workspace_id, owner_id, execution_process_id, agent_name, intent, files_json, symbols_json, dependencies_json, lease_expires_at, created_at, updated_at FROM agent_work_declarations WHERE workspace_id = ? AND status = 'active' AND lease_expires_at > ? ORDER BY updated_at DESC",
        )
        .bind(workspace_id)
        .bind(now)
        .fetch_all(pool)
        .await
        .map(|rows| rows.into_iter().map(AgentWorkRow::into_declaration).collect())
    }

    pub async fn list_active_for_repo(
        pool: &SqlitePool,
        repo_id: Uuid,
    ) -> Result<Vec<AgentWorkDeclaration>, sqlx::Error> {
        let now = Utc::now();
        Self::remove_expired(pool, now).await?;
        sqlx::query_as::<_, AgentWorkRow>(
            "SELECT awd.id, awd.workspace_id, awd.owner_id, awd.execution_process_id, awd.agent_name, awd.intent, awd.files_json, awd.symbols_json, awd.dependencies_json, awd.lease_expires_at, awd.created_at, awd.updated_at FROM agent_work_declarations awd JOIN workspace_repos wr ON wr.workspace_id = awd.workspace_id WHERE wr.repo_id = ? AND awd.status = 'active' AND awd.lease_expires_at > ? ORDER BY awd.updated_at DESC",
        )
        .bind(repo_id)
        .bind(now)
        .fetch_all(pool)
        .await
            .map(|rows| rows.into_iter().map(AgentWorkRow::into_declaration).collect())
    }

    pub async fn list_active_for_project(
        pool: &SqlitePool,
        project_id: Uuid,
    ) -> Result<Vec<AgentWorkDeclaration>, sqlx::Error> {
        let now = Utc::now();
        Self::remove_expired(pool, now).await?;
        sqlx::query_as::<_, AgentWorkRow>(
            "SELECT awd.id, awd.workspace_id, awd.owner_id, awd.execution_process_id, awd.agent_name, awd.intent, awd.files_json, awd.symbols_json, awd.dependencies_json, awd.lease_expires_at, awd.created_at, awd.updated_at FROM agent_work_declarations awd WHERE awd.status = 'active' AND awd.lease_expires_at > ? AND (EXISTS (SELECT 1 FROM workspace_repos wr JOIN project_repos pr ON pr.repo_id = wr.repo_id WHERE wr.workspace_id = awd.workspace_id AND pr.project_id = ?) OR EXISTS (SELECT 1 FROM issue_workspaces iw JOIN issues i ON i.id = iw.issue_id WHERE iw.workspace_id = awd.workspace_id AND i.project_id = ?)) ORDER BY awd.updated_at DESC",
        )
        .bind(now)
        .bind(project_id)
        .bind(project_id)
        .fetch_all(pool)
        .await
        .map(|rows| rows.into_iter().map(AgentWorkRow::into_declaration).collect())
    }

    pub async fn release_workspace(
        pool: &SqlitePool,
        workspace_id: Uuid,
    ) -> Result<u64, sqlx::Error> {
        let result = sqlx::query(
            "UPDATE agent_work_declarations SET status = 'released', updated_at = ?, lease_expires_at = ? WHERE workspace_id = ? AND status = 'active'",
        )
        .bind(Utc::now())
        .bind(Utc::now())
        .bind(workspace_id)
        .execute(pool)
        .await?;
        Ok(result.rows_affected())
    }

    async fn remove_expired(pool: &SqlitePool, now: DateTime<Utc>) -> Result<(), sqlx::Error> {
        sqlx::query(
            "DELETE FROM agent_work_declarations WHERE status = 'released' OR lease_expires_at <= ? OR (execution_process_id IS NOT NULL AND NOT EXISTS (SELECT 1 FROM execution_processes ep WHERE ep.id = agent_work_declarations.execution_process_id AND ep.status = 'running'))",
        )
        .bind(now)
        .execute(pool)
        .await
        .map(|_| ())
    }

    fn conflict(
        other: &AgentWorkDeclaration,
        input: &DeclareAgentWork,
    ) -> Option<AgentWorkConflict> {
        Self::conflict_with_scope(other, &input.files, &input.symbols, &input.dependencies)
    }

    pub fn conflict_with_scope(
        other: &AgentWorkDeclaration,
        files: &[String],
        symbols: &[String],
        dependencies: &[String],
    ) -> Option<AgentWorkConflict> {
        let file_overlap = lists_overlap(&other.files, files, path_overlaps);
        let symbol_overlap = lists_overlap(&other.symbols, symbols, symbols_overlap);
        let semantic_overlap = lists_overlap(&other.symbols, dependencies, symbols_overlap)
            || lists_overlap(&other.dependencies, symbols, symbols_overlap);

        if !file_overlap && !symbol_overlap && !semantic_overlap {
            return None;
        }

        let conflict_type = match (file_overlap, symbol_overlap, semantic_overlap) {
            (true, true, true) => "file_and_symbol_and_semantic",
            (true, true, false) => "file_and_symbol",
            (true, false, true) => "file_and_semantic",
            (true, false, false) => "file",
            (false, true, true) => "symbol_and_semantic",
            (false, true, false) => "symbol",
            (false, false, true) => "semantic",
            (false, false, false) => return None,
        };

        Some(AgentWorkConflict {
            workspace_id: other.workspace_id,
            owner_id: other.owner_id,
            agent_name: other.agent_name.clone(),
            intent: other.intent.clone(),
            files: other.files.clone(),
            symbols: other.symbols.clone(),
            dependencies: other.dependencies.clone(),
            conflict_type: conflict_type.to_string(),
            lease_expires_at: other.lease_expires_at,
        })
    }
}

fn lists_overlap<F>(left: &[String], right: &[String], overlaps: F) -> bool
where
    F: Fn(&str, &str) -> bool + Copy,
{
    left.iter()
        .any(|a| right.iter().any(|b| overlaps(a.trim(), b.trim())))
}

fn path_overlaps(left: &str, right: &str) -> bool {
    if left.is_empty() || right.is_empty() {
        return false;
    }
    if left == right {
        return true;
    }
    if let Some(prefix) = left.strip_suffix('*')
        && right.starts_with(prefix)
    {
        return true;
    }
    if let Some(prefix) = right.strip_suffix('*')
        && left.starts_with(prefix)
    {
        return true;
    }
    right.starts_with(&format!("{left}/")) || left.starts_with(&format!("{right}/"))
}

fn symbols_overlap(left: &str, right: &str) -> bool {
    if left.is_empty() || right.is_empty() || left == right {
        return !left.is_empty() && left == right;
    }
    left.starts_with(&format!("{right}::")) || right.starts_with(&format!("{left}::"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;

    async fn pool() -> SqlitePool {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        sqlx::migrate!("./migrations").run(&pool).await.unwrap();
        pool
    }

    async fn workspace(pool: &SqlitePool) -> Uuid {
        let id = Uuid::new_v4();
        sqlx::query("INSERT INTO workspaces (id, branch, name) VALUES (?, 'main', 'test')")
            .bind(id)
            .execute(pool)
            .await
            .unwrap();
        id
    }

    async fn repo_for_workspaces(pool: &SqlitePool, workspace_ids: &[Uuid]) -> Uuid {
        let repo_id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO repos (id, path, name, display_name) VALUES (?, '/tmp/test-repo', 'test-repo', 'test-repo')",
        )
        .bind(repo_id)
        .execute(pool)
        .await
        .unwrap();
        for workspace_id in workspace_ids {
            sqlx::query(
                "INSERT INTO workspace_repos (id, workspace_id, repo_id, target_branch) VALUES (?, ?, ?, 'main')",
            )
            .bind(Uuid::new_v4())
            .bind(workspace_id)
            .bind(repo_id)
            .execute(pool)
            .await
            .unwrap();
        }
        repo_id
    }

    fn declaration_input(workspace_id: Uuid, owner_id: Uuid) -> DeclareAgentWork {
        DeclareAgentWork {
            workspace_id,
            owner_id,
            execution_process_id: None,
            agent_name: "agent-a".to_string(),
            intent: "Update merge coordination".to_string(),
            files: vec!["crates/git/src/lib.rs".to_string()],
            symbols: vec!["merge_changes".to_string()],
            dependencies: vec![],
        }
    }

    #[tokio::test]
    async fn declaration_reports_file_and_symbol_overlap_without_blocking() {
        let pool = pool().await;
        let workspace_id = workspace(&pool).await;
        let first = declaration_input(workspace_id, Uuid::new_v4());
        AgentWorkDeclaration::declare(&pool, &first).await.unwrap();

        let mut second = declaration_input(workspace_id, Uuid::new_v4());
        second.agent_name = "agent-b".to_string();
        second.intent = "Refactor the same merge path".to_string();
        let result = AgentWorkDeclaration::declare(&pool, &second).await.unwrap();

        assert_eq!(result.conflicts.len(), 1);
        assert_eq!(result.conflicts[0].conflict_type, "file_and_symbol");
        assert_eq!(
            AgentWorkDeclaration::list_active(&pool, workspace_id)
                .await
                .unwrap()
                .len(),
            2
        );
    }

    #[tokio::test]
    async fn release_removes_declaration_from_active_view() {
        let pool = pool().await;
        let workspace_id = workspace(&pool).await;
        let owner_id = Uuid::new_v4();
        AgentWorkDeclaration::declare(&pool, &declaration_input(workspace_id, owner_id))
            .await
            .unwrap();

        assert!(
            AgentWorkDeclaration::release(&pool, workspace_id, owner_id)
                .await
                .unwrap()
        );
        assert!(
            AgentWorkDeclaration::list_active(&pool, workspace_id)
                .await
                .unwrap()
                .is_empty()
        );
    }

    #[tokio::test]
    async fn repo_scope_lists_active_declarations_across_workspaces() {
        let pool = pool().await;
        let first_workspace = workspace(&pool).await;
        let second_workspace = workspace(&pool).await;
        let repo_id = repo_for_workspaces(&pool, &[first_workspace, second_workspace]).await;

        AgentWorkDeclaration::declare(&pool, &declaration_input(first_workspace, Uuid::new_v4()))
            .await
            .unwrap();
        AgentWorkDeclaration::declare(&pool, &declaration_input(second_workspace, Uuid::new_v4()))
            .await
            .unwrap();

        assert_eq!(
            AgentWorkDeclaration::list_active_for_repo(&pool, repo_id)
                .await
                .unwrap()
                .len(),
            2
        );
    }

    #[tokio::test]
    async fn project_scope_includes_issue_linked_workspaces() {
        let pool = pool().await;
        let workspace_id = workspace(&pool).await;
        let project_id = Uuid::new_v4();
        let status_id = Uuid::new_v4();
        let issue_id = Uuid::new_v4();

        sqlx::query(
            "INSERT INTO projects (id, name, color, sort_order) VALUES (?, 'P', '#fff', 0)",
        )
        .bind(project_id)
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO project_statuses (id, project_id, name, color) VALUES (?, ?, 'Todo', '#fff')",
        )
        .bind(status_id)
        .bind(project_id)
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO issues (id, project_id, issue_number, simple_id, status_id, title) VALUES (?, ?, 1, 'P-1', ?, 'T')",
        )
        .bind(issue_id)
        .bind(project_id)
        .bind(status_id)
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query("INSERT INTO issue_workspaces (id, issue_id, workspace_id) VALUES (?, ?, ?)")
            .bind(Uuid::new_v4())
            .bind(issue_id)
            .bind(workspace_id)
            .execute(&pool)
            .await
            .unwrap();

        AgentWorkDeclaration::declare(&pool, &declaration_input(workspace_id, Uuid::new_v4()))
            .await
            .unwrap();

        assert_eq!(
            AgentWorkDeclaration::list_active_for_project(&pool, project_id)
                .await
                .unwrap()
                .len(),
            1
        );
    }

    #[tokio::test]
    async fn inactive_execution_declaration_is_not_listed() {
        let pool = pool().await;
        let workspace_id = workspace(&pool).await;
        let session_id = Uuid::new_v4();
        let execution_id = Uuid::new_v4();
        sqlx::query("INSERT INTO sessions (id, workspace_id) VALUES (?, ?)")
            .bind(session_id)
            .bind(workspace_id)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query(
            "INSERT INTO execution_processes (id, session_id, run_reason, executor_action, status) VALUES (?, ?, 'codingagent', '{}', 'failed')",
        )
        .bind(execution_id)
        .bind(session_id)
        .execute(&pool)
        .await
        .unwrap();

        let mut input = declaration_input(workspace_id, execution_id);
        input.execution_process_id = Some(execution_id);
        AgentWorkDeclaration::declare(&pool, &input).await.unwrap();

        assert!(
            AgentWorkDeclaration::list_active(&pool, workspace_id)
                .await
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn dependency_overlap_is_reported_as_semantic_conflict() {
        let workspace_id = Uuid::new_v4();
        let mut owner = declaration_input(workspace_id, Uuid::new_v4());
        owner.symbols = vec!["git::merge_changes".to_string()];
        let declaration = AgentWorkDeclaration {
            id: Uuid::new_v4(),
            workspace_id,
            owner_id: owner.owner_id,
            execution_process_id: None,
            agent_name: owner.agent_name,
            intent: owner.intent,
            files: owner.files,
            symbols: owner.symbols,
            dependencies: vec![],
            lease_expires_at: Utc::now() + Duration::minutes(1),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let conflict = AgentWorkDeclaration::conflict_with_scope(
            &declaration,
            &["crates/other/src/caller.rs".to_string()],
            &[],
            &["git::merge_changes".to_string()],
        )
        .expect("dependency overlap should be reported");

        assert_eq!(conflict.conflict_type, "semantic");
    }
}
