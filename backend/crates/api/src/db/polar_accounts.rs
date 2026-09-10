use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct PolarAccountRecord {
    pub id: Uuid,
    pub app_user_id: Uuid,
    pub polar_user_id: i64,
    /// Salattu access token, ks. `crate::crypto`.
    pub access_token_enc: Vec<u8>,
    pub member_id: String,
    pub registered_at: DateTime<Utc>,
    pub last_sync_at: Option<DateTime<Utc>>,
}

/// Luo tai korvaa käyttäjän Polar-tilin. Uudelleenyhdistäminen samalle
/// käyttäjälle päivittää tokenin ja nollaa `last_sync_at`:n.
pub async fn upsert_for_user(
    pool: &PgPool,
    app_user_id: Uuid,
    polar_user_id: i64,
    access_token_enc: &[u8],
    member_id: &str,
) -> sqlx::Result<PolarAccountRecord> {
    sqlx::query_as!(
        PolarAccountRecord,
        r#"
        INSERT INTO polar_accounts (app_user_id, polar_user_id, access_token_enc, member_id)
        VALUES ($1, $2, $3, $4)
        ON CONFLICT (app_user_id) DO UPDATE
            SET polar_user_id    = EXCLUDED.polar_user_id,
                access_token_enc = EXCLUDED.access_token_enc,
                member_id        = EXCLUDED.member_id,
                registered_at    = now(),
                last_sync_at     = NULL
        RETURNING id, app_user_id, polar_user_id, access_token_enc, member_id,
                  registered_at, last_sync_at
        "#,
        app_user_id,
        polar_user_id,
        access_token_enc,
        member_id
    )
    .fetch_one(pool)
    .await
}

pub async fn find_by_user(
    pool: &PgPool,
    app_user_id: Uuid,
) -> sqlx::Result<Option<PolarAccountRecord>> {
    sqlx::query_as!(
        PolarAccountRecord,
        "SELECT id, app_user_id, polar_user_id, access_token_enc, member_id,
                registered_at, last_sync_at
         FROM polar_accounts WHERE app_user_id = $1",
        app_user_id
    )
    .fetch_optional(pool)
    .await
}

pub async fn list_all(pool: &PgPool) -> sqlx::Result<Vec<PolarAccountRecord>> {
    sqlx::query_as!(
        PolarAccountRecord,
        "SELECT id, app_user_id, polar_user_id, access_token_enc, member_id,
                registered_at, last_sync_at
         FROM polar_accounts ORDER BY registered_at"
    )
    .fetch_all(pool)
    .await
}

/// Palauttaa poistettujen rivien määrän (0 tai 1).
pub async fn delete_by_user(pool: &PgPool, app_user_id: Uuid) -> sqlx::Result<u64> {
    let result = sqlx::query!(
        "DELETE FROM polar_accounts WHERE app_user_id = $1",
        app_user_id
    )
    .execute(pool)
    .await?;
    Ok(result.rows_affected())
}

pub async fn set_last_sync_at(pool: &PgPool, account_id: Uuid) -> sqlx::Result<()> {
    sqlx::query!(
        "UPDATE polar_accounts SET last_sync_at = now() WHERE id = $1",
        account_id
    )
    .execute(pool)
    .await?;
    Ok(())
}
