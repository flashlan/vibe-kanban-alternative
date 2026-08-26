use chrono::{DateTime, Duration, Utc};
use sqlx::SqlitePool;
use uuid::Uuid;

/// The Integration Guard lease only covers the short validation + Git write
/// section. A crashed backend cannot hold the repository indefinitely.
pub const INTEGRATION_GUARD_LEASE_SECONDS: i64 = 5 * 60;

#[derive(Debug, Clone)]
pub struct IntegrationGuardLease {
    pub repo_id: Uuid,
    pub owner_id: Uuid,
    pub lease_expires_at: DateTime<Utc>,
}

impl IntegrationGuardLease {
    pub async fn try_acquire(
        pool: &SqlitePool,
        repo_id: Uuid,
        owner_id: Uuid,
    ) -> Result<Option<Self>, sqlx::Error> {
        let now = Utc::now();
        let lease_expires_at = now + Duration::seconds(INTEGRATION_GUARD_LEASE_SECONDS);
        let mut transaction = pool.begin().await?;

        sqlx::query("DELETE FROM integration_guard_locks WHERE lease_expires_at <= ?")
            .bind(now)
            .execute(&mut *transaction)
            .await?;

        let inserted = sqlx::query(
            "INSERT INTO integration_guard_locks (repo_id, owner_id, lease_expires_at) VALUES (?, ?, ?) ON CONFLICT (repo_id) DO NOTHING",
        )
        .bind(repo_id)
        .bind(owner_id)
        .bind(lease_expires_at)
        .execute(&mut *transaction)
        .await?
        .rows_affected();

        transaction.commit().await?;

        Ok((inserted > 0).then_some(Self {
            repo_id,
            owner_id,
            lease_expires_at,
        }))
    }

    pub async fn release(
        pool: &SqlitePool,
        repo_id: Uuid,
        owner_id: Uuid,
    ) -> Result<bool, sqlx::Error> {
        let result =
            sqlx::query("DELETE FROM integration_guard_locks WHERE repo_id = ? AND owner_id = ?")
                .bind(repo_id)
                .bind(owner_id)
                .execute(pool)
                .await?;
        Ok(result.rows_affected() > 0)
    }
}

#[cfg(test)]
mod tests {
    use sqlx::sqlite::SqlitePoolOptions;

    use super::*;

    async fn pool() -> SqlitePool {
        let pool = SqlitePoolOptions::new()
            .max_connections(2)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        sqlx::migrate!("./migrations").run(&pool).await.unwrap();
        pool
    }

    async fn repo(pool: &SqlitePool) -> Uuid {
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO repos (id, path, name, display_name) VALUES (?, '/tmp/guard-repo', 'guard-repo', 'guard-repo')",
        )
        .bind(id)
        .execute(pool)
        .await
        .unwrap();
        id
    }

    #[tokio::test]
    async fn only_one_owner_can_acquire_a_repository_guard() {
        let pool = pool().await;
        let repo_id = repo(&pool).await;
        let first_owner = Uuid::new_v4();
        let second_owner = Uuid::new_v4();

        assert!(
            IntegrationGuardLease::try_acquire(&pool, repo_id, first_owner)
                .await
                .unwrap()
                .is_some()
        );
        assert!(
            IntegrationGuardLease::try_acquire(&pool, repo_id, second_owner)
                .await
                .unwrap()
                .is_none()
        );
        assert!(
            IntegrationGuardLease::release(&pool, repo_id, first_owner)
                .await
                .unwrap()
        );
        assert!(
            IntegrationGuardLease::try_acquire(&pool, repo_id, second_owner)
                .await
                .unwrap()
                .is_some()
        );
    }
}
