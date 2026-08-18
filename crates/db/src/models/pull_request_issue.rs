use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PullRequestIssue {
    pub id: String,
    pub pull_request_id: String,
    pub issue_id: Uuid,
    pub created_at: DateTime<Utc>,
}

impl PullRequestIssue {
    pub async fn list_by_project(
        pool: &SqlitePool,
        project_id: Uuid,
    ) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as!(
            PullRequestIssue,
            r#"SELECT pri.id,
                      pri.pull_request_id,
                      pri.issue_id as "issue_id!: Uuid",
                      pri.created_at as "created_at!: DateTime<Utc>"
               FROM pull_request_issues pri
               INNER JOIN issues i ON i.id = pri.issue_id
               WHERE i.project_id = $1
               ORDER BY pri.created_at ASC"#,
            project_id
        )
        .fetch_all(pool)
        .await
    }
}
