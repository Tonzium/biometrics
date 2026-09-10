use chrono::{DateTime, Utc};
use serde::Serialize;
use serde_json::Value;
use sqlx::PgPool;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct SyncRunRecord {
    pub id: i64,
    #[serde(skip)]
    pub polar_account_id: Uuid,
    pub trigger: String,
    pub status: String,
    pub started_at: DateTime<Utc>,
    pub finished_at: Option<DateTime<Utc>>,
    /// Upsertattujen rivien määrät datatyypeittäin.
    #[schema(value_type = Object)]
    pub counts: Value,
    pub error: Option<String>,
}

pub async fn start(pool: &PgPool, polar_account_id: Uuid, trigger: &str) -> sqlx::Result<i64> {
    sqlx::query_scalar!(
        "INSERT INTO sync_runs (polar_account_id, trigger) VALUES ($1, $2) RETURNING id",
        polar_account_id,
        trigger
    )
    .fetch_one(pool)
    .await
}

pub async fn finish(
    pool: &PgPool,
    id: i64,
    status: &str,
    counts: &Value,
    error: Option<&str>,
) -> sqlx::Result<()> {
    sqlx::query!(
        "UPDATE sync_runs
         SET status = $2, counts = $3, error = $4, finished_at = now()
         WHERE id = $1",
        id,
        status,
        counts,
        error
    )
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn list_recent(
    pool: &PgPool,
    polar_account_id: Uuid,
    limit: i64,
) -> sqlx::Result<Vec<SyncRunRecord>> {
    sqlx::query_as!(
        SyncRunRecord,
        "SELECT id, polar_account_id, trigger, status, started_at, finished_at, counts, error
         FROM sync_runs WHERE polar_account_id = $1
         ORDER BY started_at DESC LIMIT $2",
        polar_account_id,
        limit
    )
    .fetch_all(pool)
    .await
}
