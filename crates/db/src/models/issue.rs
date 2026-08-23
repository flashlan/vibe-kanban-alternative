use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::SqlitePool;
use ts_rs::TS;
use uuid::Uuid;

use crate::models::project_status::ProjectStatus;

/// Kanban card. Mirrors the wire `Issue` shape consumed by the frontend
/// (served at /v1/fallback/issues, mutated at /v1/issues).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Issue {
    pub id: Uuid,
    pub project_id: Uuid,
    pub issue_number: i64,
    pub simple_id: String,
    pub status_id: Uuid,
    pub title: String,
    pub description: Option<String>,
    /// Serialised IssuePriority value ("urgent" | "high" | "medium" | "low").
    pub priority: Option<String>,
    pub start_date: Option<DateTime<Utc>>,
    pub target_date: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub sort_order: f64,
    pub parent_issue_id: Option<Uuid>,
    pub parent_issue_sort_order: Option<f64>,
    pub extension_metadata: Value,
    pub archived: bool,
    pub archived_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Row representation: `extension_metadata` is stored as a JSON TEXT column.
struct IssueRow {
    id: Uuid,
    project_id: Uuid,
    issue_number: i64,
    simple_id: String,
    status_id: Uuid,
    title: String,
    description: Option<String>,
    priority: Option<String>,
    start_date: Option<DateTime<Utc>>,
    target_date: Option<DateTime<Utc>>,
    completed_at: Option<DateTime<Utc>>,
    sort_order: f64,
    parent_issue_id: Option<Uuid>,
    parent_issue_sort_order: Option<f64>,
    extension_metadata: String,
    archived: bool,
    archived_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<IssueRow> for Issue {
    fn from(r: IssueRow) -> Self {
        Issue {
            id: r.id,
            project_id: r.project_id,
            issue_number: r.issue_number,
            simple_id: r.simple_id,
            status_id: r.status_id,
            title: r.title,
            description: r.description,
            priority: r.priority,
            start_date: r.start_date,
            target_date: r.target_date,
            completed_at: r.completed_at,
            sort_order: r.sort_order,
            parent_issue_id: r.parent_issue_id,
            parent_issue_sort_order: r.parent_issue_sort_order,
            extension_metadata: serde_json::from_str(&r.extension_metadata)
                .unwrap_or(Value::Object(Default::default())),
            archived: r.archived,
            archived_at: r.archived_at,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

/// Full field set for creating an issue. `id` may be client-generated.
pub struct NewIssue<'a> {
    pub id: Uuid,
    pub project_id: Uuid,
    pub status_id: Uuid,
    pub title: &'a str,
    pub description: Option<&'a str>,
    pub priority: Option<&'a str>,
    pub start_date: Option<DateTime<Utc>>,
    pub target_date: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub sort_order: f64,
    pub parent_issue_id: Option<Uuid>,
    pub parent_issue_sort_order: Option<f64>,
    pub extension_metadata: &'a str,
    /// Project issue prefix used to build `simple_id`.
    pub key: &'a str,
}

/// Full new state for an update (the route merges partial requests first).
pub struct IssueUpdate<'a> {
    pub status_id: Uuid,
    pub title: &'a str,
    pub description: Option<&'a str>,
    pub priority: Option<&'a str>,
    pub start_date: Option<DateTime<Utc>>,
    pub target_date: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub sort_order: f64,
    pub parent_issue_id: Option<Uuid>,
    pub parent_issue_sort_order: Option<f64>,
    pub extension_metadata: &'a str,
}

impl Issue {
    pub async fn list_by_project(
        pool: &SqlitePool,
        project_id: Uuid,
    ) -> Result<Vec<Self>, sqlx::Error> {
        let rows = sqlx::query_as!(
            IssueRow,
            r#"SELECT id as "id!: Uuid",
                      project_id as "project_id!: Uuid",
                      issue_number,
                      simple_id,
                      status_id as "status_id!: Uuid",
                      title,
                      description,
                      priority,
                      start_date as "start_date: DateTime<Utc>",
                      target_date as "target_date: DateTime<Utc>",
                      completed_at as "completed_at: DateTime<Utc>",
                      sort_order as "sort_order!: f64",
                      parent_issue_id as "parent_issue_id: Uuid",
                      parent_issue_sort_order as "parent_issue_sort_order: f64",
                      extension_metadata,
                      archived as "archived!: bool",
                      archived_at as "archived_at: DateTime<Utc>",
                      created_at as "created_at!: DateTime<Utc>",
                      updated_at as "updated_at!: DateTime<Utc>"
               FROM issues
               WHERE project_id = $1 AND COALESCE(archived, 0) = 0
               ORDER BY sort_order ASC"#,
            project_id
        )
        .fetch_all(pool)
        .await?;
        Ok(rows.into_iter().map(Issue::from).collect())
    }

    /// Archived issues for a project (hidden from the active board). Used by the
    /// archive recovery view so they can be restored or permanently deleted.
    pub async fn list_archived_by_project(
        pool: &SqlitePool,
        project_id: Uuid,
    ) -> Result<Vec<Self>, sqlx::Error> {
        let rows = sqlx::query_as!(
            IssueRow,
            r#"SELECT id as "id!: Uuid",
                      project_id as "project_id!: Uuid",
                      issue_number,
                      simple_id,
                      status_id as "status_id!: Uuid",
                      title,
                      description,
                      priority,
                      start_date as "start_date: DateTime<Utc>",
                      target_date as "target_date: DateTime<Utc>",
                      completed_at as "completed_at: DateTime<Utc>",
                      sort_order as "sort_order!: f64",
                      parent_issue_id as "parent_issue_id: Uuid",
                      parent_issue_sort_order as "parent_issue_sort_order: f64",
                      extension_metadata,
                      archived as "archived!: bool",
                      archived_at as "archived_at: DateTime<Utc>",
                      created_at as "created_at!: DateTime<Utc>",
                      updated_at as "updated_at!: DateTime<Utc>"
               FROM issues
               WHERE project_id = $1 AND archived = 1
               ORDER BY archived_at DESC"#,
            project_id
        )
        .fetch_all(pool)
        .await?;
        Ok(rows.into_iter().map(Issue::from).collect())
    }

    pub async fn find_by_id(pool: &SqlitePool, id: Uuid) -> Result<Option<Self>, sqlx::Error> {
        let row = sqlx::query_as!(
            IssueRow,
            r#"SELECT id as "id!: Uuid",
                      project_id as "project_id!: Uuid",
                      issue_number,
                      simple_id,
                      status_id as "status_id!: Uuid",
                      title,
                      description,
                      priority,
                      start_date as "start_date: DateTime<Utc>",
                      target_date as "target_date: DateTime<Utc>",
                      completed_at as "completed_at: DateTime<Utc>",
                      sort_order as "sort_order!: f64",
                      parent_issue_id as "parent_issue_id: Uuid",
                      parent_issue_sort_order as "parent_issue_sort_order: f64",
                      extension_metadata,
                      archived as "archived!: bool",
                      archived_at as "archived_at: DateTime<Utc>",
                      created_at as "created_at!: DateTime<Utc>",
                      updated_at as "updated_at!: DateTime<Utc>"
               FROM issues
               WHERE id = $1"#,
            id
        )
        .fetch_optional(pool)
        .await?;
        Ok(row.map(Issue::from))
    }

    pub async fn create(pool: &SqlitePool, new: NewIssue<'_>) -> Result<Self, sqlx::Error> {
        let next = sqlx::query!(
            r#"SELECT COALESCE(MAX(issue_number), 0) + 1 as "n!: i64"
               FROM issues WHERE project_id = $1"#,
            new.project_id
        )
        .fetch_one(pool)
        .await?;
        let issue_number = next.n;
        let simple_id = format!("{}-{}", new.key, issue_number);

        let row = sqlx::query_as!(
            IssueRow,
            r#"INSERT INTO issues (
                   id, project_id, issue_number, simple_id, status_id, title,
                   description, priority, start_date, target_date, completed_at,
                   sort_order, parent_issue_id, parent_issue_sort_order,
                   extension_metadata
               ) VALUES (
                   $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15
               )
               RETURNING id as "id!: Uuid",
                         project_id as "project_id!: Uuid",
                         issue_number,
                         simple_id,
                         status_id as "status_id!: Uuid",
                         title,
                         description,
                         priority,
                         start_date as "start_date: DateTime<Utc>",
                         target_date as "target_date: DateTime<Utc>",
                         completed_at as "completed_at: DateTime<Utc>",
                         sort_order as "sort_order!: f64",
                          parent_issue_id as "parent_issue_id: Uuid",
                          parent_issue_sort_order as "parent_issue_sort_order: f64",
                          extension_metadata,
                          archived as "archived!: bool",
                          archived_at as "archived_at: DateTime<Utc>",
                          created_at as "created_at!: DateTime<Utc>",
                          updated_at as "updated_at!: DateTime<Utc>""#,
            new.id,
            new.project_id,
            issue_number,
            simple_id,
            new.status_id,
            new.title,
            new.description,
            new.priority,
            new.start_date,
            new.target_date,
            new.completed_at,
            new.sort_order,
            new.parent_issue_id,
            new.parent_issue_sort_order,
            new.extension_metadata,
        )
        .fetch_one(pool)
        .await?;
        Ok(Issue::from(row))
    }

    pub async fn update(
        pool: &SqlitePool,
        id: Uuid,
        u: IssueUpdate<'_>,
    ) -> Result<Option<Self>, sqlx::Error> {
        let row = sqlx::query_as!(
            IssueRow,
            r#"UPDATE issues
               SET status_id = $2,
                   title = $3,
                   description = $4,
                   priority = $5,
                   start_date = $6,
                   target_date = $7,
                   completed_at = $8,
                   sort_order = $9,
                   parent_issue_id = $10,
                   parent_issue_sort_order = $11,
                   extension_metadata = $12,
                   updated_at = datetime('now', 'subsec')
               WHERE id = $1
               RETURNING id as "id!: Uuid",
                         project_id as "project_id!: Uuid",
                         issue_number,
                         simple_id,
                         status_id as "status_id!: Uuid",
                         title,
                         description,
                         priority,
                         start_date as "start_date: DateTime<Utc>",
                         target_date as "target_date: DateTime<Utc>",
                         completed_at as "completed_at: DateTime<Utc>",
                         sort_order as "sort_order!: f64",
                          parent_issue_id as "parent_issue_id: Uuid",
                          parent_issue_sort_order as "parent_issue_sort_order: f64",
                          extension_metadata,
                          archived as "archived!: bool",
                          archived_at as "archived_at: DateTime<Utc>",
                          created_at as "created_at!: DateTime<Utc>",
                          updated_at as "updated_at!: DateTime<Utc>""#,
            id,
            u.status_id,
            u.title,
            u.description,
            u.priority,
            u.start_date,
            u.target_date,
            u.completed_at,
            u.sort_order,
            u.parent_issue_id,
            u.parent_issue_sort_order,
            u.extension_metadata,
        )
        .fetch_optional(pool)
        .await?;
        Ok(row.map(Issue::from))
    }

    pub async fn delete(pool: &SqlitePool, id: Uuid) -> Result<u64, sqlx::Error> {
        let result = sqlx::query!("DELETE FROM issues WHERE id = $1", id)
            .execute(pool)
            .await?;
        Ok(result.rows_affected())
    }

    /// Move an issue into the archive (soft delete). Sets `archived = 1` and
    /// stamps `archived_at`; the active board no longer lists it.
    pub async fn archive(pool: &SqlitePool, id: Uuid) -> Result<u64, sqlx::Error> {
        let result = sqlx::query!(
            r#"UPDATE issues
               SET archived = 1,
                   archived_at = datetime('now', 'subsec'),
                   updated_at = datetime('now', 'subsec')
               WHERE id = $1"#,
            id
        )
        .execute(pool)
        .await?;
        Ok(result.rows_affected())
    }

    /// Restore an archived issue back to the active board. Clears `archived`
    /// and `archived_at`.
    pub async fn restore(pool: &SqlitePool, id: Uuid) -> Result<u64, sqlx::Error> {
        let result = sqlx::query!(
            r#"UPDATE issues
                SET archived = 0,
                    archived_at = NULL,
                    updated_at = datetime('now', 'subsec')
                WHERE id = $1"#,
            id
        )
        .execute(pool)
        .await?;
        Ok(result.rows_affected())
    }
}

/// Semantically classified role of a kanban column, used to derive lifecycle
/// metrics (review cycles, rework) from the raw status-change history.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StatusRole {
    Todo,
    InProgress,
    Review,
    Done,
    Other,
}

fn status_role(name: &str, is_terminal: bool) -> StatusRole {
    if is_terminal {
        return StatusRole::Done;
    }
    let n = name.to_lowercase();
    if n.contains("review") || n.contains("qa") {
        StatusRole::Review
    } else if n.contains("progress") || n.contains("doing") || n.contains("wip") {
        StatusRole::InProgress
    } else if n.contains("todo") || n.contains("backlog") || n.contains("to do") {
        StatusRole::Todo
    } else {
        StatusRole::Other
    }
}

/// Lifecycle metrics for a single card, derived from `issue_status_history`.
#[derive(Debug, Clone, Serialize, TS)]
pub struct IssueMetrics {
    pub issue_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    /// Total elapsed seconds: `completed_at - created_at` when done, else
    /// `now - created_at`.
    pub total_seconds: i64,
    /// Number of `in_progress → review` transitions (each review entry = a cycle).
    pub cycles: i64,
    /// Number of `review → in_progress` transitions (each return = rework).
    pub rework_count: i64,
    /// Number of recorded status transitions.
    pub status_changes: i64,
    pub current_status_name: String,
}

impl Issue {
    /// Compute lifecycle metrics for `issue_id` from its status-change history.
    /// Returns `None` when the issue does not exist.
    pub async fn metrics(
        pool: &SqlitePool,
        issue_id: Uuid,
    ) -> Result<Option<IssueMetrics>, sqlx::Error> {
        let issue = match Self::find_by_id(pool, issue_id).await? {
            Some(i) => i,
            None => return Ok(None),
        };

        let history = sqlx::query!(
            r#"SELECT from_status_id as "from_status_id?: Uuid",
                      to_status_id as "to_status_id!: Uuid"
               FROM issue_status_history
               WHERE issue_id = $1
               ORDER BY created_at ASC, rowid ASC"#,
            issue_id
        )
        .fetch_all(pool)
        .await?;

        let statuses = ProjectStatus::list_by_project(pool, issue.project_id).await?;
        let role_of = |id: Uuid| -> StatusRole {
            statuses
                .iter()
                .find(|s| s.id == id)
                .map(|s| status_role(&s.name, s.is_terminal))
                .unwrap_or(StatusRole::Other)
        };

        // Reconstruct the sequence of status roles visited by the card, starting
        // from its first status (the from_status of the earliest change).
        let mut visited: Vec<StatusRole> = Vec::new();
        if history.is_empty() {
            visited.push(role_of(issue.status_id));
        } else {
            visited.push(role_of(
                history[0].from_status_id.unwrap_or(issue.status_id),
            ));
            for row in &history {
                visited.push(role_of(row.to_status_id));
            }
        }

        let mut cycles = 0i64;
        let mut rework = 0i64;
        for window in visited.windows(2) {
            match (window[0], window[1]) {
                (StatusRole::InProgress, StatusRole::Review) => cycles += 1,
                (StatusRole::Review, StatusRole::InProgress) => rework += 1,
                _ => {}
            }
        }

        let end = issue.completed_at.unwrap_or_else(Utc::now);
        let total_seconds = (end - issue.created_at).num_seconds();

        let current_status_name = statuses
            .iter()
            .find(|s| s.id == issue.status_id)
            .map(|s| s.name.clone())
            .unwrap_or_default();

        Ok(Some(IssueMetrics {
            issue_id,
            created_at: issue.created_at,
            completed_at: issue.completed_at,
            total_seconds,
            cycles,
            rework_count: rework,
            status_changes: history.len() as i64,
            current_status_name,
        }))
    }
}

#[cfg(test)]
mod metrics_tests {
    use super::*;
    use sqlx::{SqlitePool, migrate::Migrator};
    use uuid::Uuid;

    async fn migrated_pool() -> SqlitePool {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        let migrator = Migrator::new(std::path::Path::new("./migrations"))
            .await
            .unwrap();
        migrator.run(&pool).await.unwrap();
        pool
    }

    async fn insert_project(pool: &SqlitePool, id: Uuid) {
        sqlx::query("INSERT INTO projects (id, name, color) VALUES (?, ?, '#fff')")
            .bind(id)
            .bind("P")
            .execute(pool)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn counts_cycles_and_rework_from_history() {
        let pool = migrated_pool().await;
        let project_id = Uuid::new_v4();
        insert_project(&pool, project_id).await;

        let todo = ProjectStatus::create(
            &pool,
            Uuid::new_v4(),
            project_id,
            "Todo",
            "#fff",
            0,
            false,
            false,
        )
        .await
        .unwrap();
        let in_progress = ProjectStatus::create(
            &pool,
            Uuid::new_v4(),
            project_id,
            "In Progress",
            "#fff",
            1,
            false,
            false,
        )
        .await
        .unwrap();
        let in_review = ProjectStatus::create(
            &pool,
            Uuid::new_v4(),
            project_id,
            "In Review",
            "#fff",
            2,
            false,
            false,
        )
        .await
        .unwrap();
        let done = ProjectStatus::create(
            &pool,
            Uuid::new_v4(),
            project_id,
            "Done",
            "#fff",
            3,
            false,
            true,
        )
        .await
        .unwrap();

        let issue = Issue::create(
            &pool,
            NewIssue {
                id: Uuid::new_v4(),
                project_id,
                status_id: todo.id,
                title: "Card",
                description: None,
                priority: None,
                start_date: None,
                target_date: None,
                completed_at: None,
                sort_order: 0.0,
                parent_issue_id: None,
                parent_issue_sort_order: None,
                extension_metadata: "{}",
                key: "TST",
            },
        )
        .await
        .unwrap();

        // Simulate the lifecycle: Todo -> In Progress -> In Review ->
        // In Progress (rework) -> In Review -> Done.
        for status in [&in_progress, &in_review, &in_progress, &in_review, &done] {
            sqlx::query("UPDATE issues SET status_id = $1 WHERE id = $2")
                .bind(status.id)
                .bind(issue.id)
                .execute(&pool)
                .await
                .unwrap();
        }

        let metrics = Issue::metrics(&pool, issue.id).await.unwrap().unwrap();
        assert_eq!(metrics.cycles, 2, "expected 2 review cycles");
        assert_eq!(metrics.rework_count, 1, "expected 1 rework");
        assert_eq!(metrics.status_changes, 5);
        assert_eq!(metrics.current_status_name, "Done");
        assert!(metrics.total_seconds >= 0);
    }

    #[tokio::test]
    async fn no_history_yields_zero_cycles() {
        let pool = migrated_pool().await;
        let project_id = Uuid::new_v4();
        insert_project(&pool, project_id).await;
        let todo = ProjectStatus::create(
            &pool,
            Uuid::new_v4(),
            project_id,
            "Todo",
            "#fff",
            0,
            false,
            false,
        )
        .await
        .unwrap();

        let issue = Issue::create(
            &pool,
            NewIssue {
                id: Uuid::new_v4(),
                project_id,
                status_id: todo.id,
                title: "Card",
                description: None,
                priority: None,
                start_date: None,
                target_date: None,
                completed_at: None,
                sort_order: 0.0,
                parent_issue_id: None,
                parent_issue_sort_order: None,
                extension_metadata: "{}",
                key: "TST",
            },
        )
        .await
        .unwrap();

        let metrics = Issue::metrics(&pool, issue.id).await.unwrap().unwrap();
        assert_eq!(metrics.cycles, 0);
        assert_eq!(metrics.rework_count, 0);
        assert_eq!(metrics.status_changes, 0);
    }
}
