use chrono::{DateTime, Utc};
use domain::{Role, User};
use sqlx::PgPool;
use uuid::Uuid;

/// Kantarivi salasanatiivisteineen. Ei koskaan sarjallisteta ulos.
#[derive(Debug)]
pub struct UserRecord {
    pub id: Uuid,
    pub email: String,
    pub password_hash: String,
    pub role: String,
    pub created_at: DateTime<Utc>,
}

impl UserRecord {
    pub fn into_user(self) -> User {
        User {
            id: self.id,
            email: self.email,
            // Rooli on rajoitettu CHECK-ehdolla kannassa, joten tuntematon arvo
            // olisi ohjelmointivirhe; varaudutaan silti turvallisella oletuksella.
            role: Role::parse(&self.role).unwrap_or(Role::Viewer),
            created_at: self.created_at,
        }
    }
}

pub async fn count(pool: &PgPool) -> sqlx::Result<i64> {
    sqlx::query_scalar!(r#"SELECT count(*) AS "count!" FROM app_users"#)
        .fetch_one(pool)
        .await
}

pub async fn find_by_email(pool: &PgPool, email: &str) -> sqlx::Result<Option<UserRecord>> {
    sqlx::query_as!(
        UserRecord,
        "SELECT id, email, password_hash, role, created_at FROM app_users WHERE email = $1",
        email.to_lowercase()
    )
    .fetch_optional(pool)
    .await
}

pub async fn find_by_id(pool: &PgPool, id: Uuid) -> sqlx::Result<Option<UserRecord>> {
    sqlx::query_as!(
        UserRecord,
        "SELECT id, email, password_hash, role, created_at FROM app_users WHERE id = $1",
        id
    )
    .fetch_optional(pool)
    .await
}

pub async fn insert(
    pool: &PgPool,
    email: &str,
    password_hash: &str,
    role: Role,
) -> sqlx::Result<User> {
    let record = sqlx::query_as!(
        UserRecord,
        "INSERT INTO app_users (email, password_hash, role)
         VALUES ($1, $2, $3)
         RETURNING id, email, password_hash, role, created_at",
        email.to_lowercase(),
        password_hash,
        role.as_str()
    )
    .fetch_one(pool)
    .await?;
    Ok(record.into_user())
}
