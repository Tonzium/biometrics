//! Ensimmäisen käynnistyksen alustus: omistajakäyttäjä ympäristömuuttujista.
//!
//! Rekisteröintiä ei ole, joten ainoa tapa saada ensimmäinen käyttäjä on
//! `ADMIN_EMAIL` + `ADMIN_PASSWORD`. Kun kannassa on käyttäjiä, näitä
//! muuttujia ei enää lueta, joten niiden voi poistaa `.env`:stä.

use anyhow::{Context, bail};
use domain::Role;
use sqlx::PgPool;

use crate::{auth::password, config::Config, db};

pub async fn ensure_owner(pool: &PgPool, config: &Config) -> anyhow::Result<()> {
    if db::users::count(pool).await.context("counting users")? > 0 {
        return Ok(());
    }

    let (Some(email), Some(pw)) = (&config.admin_email, &config.admin_password) else {
        bail!(
            "no users exist and ADMIN_EMAIL / ADMIN_PASSWORD are not set; \
             cannot create the first owner account"
        );
    };
    if pw.len() < 12 {
        bail!("ADMIN_PASSWORD must be at least 12 characters");
    }

    let hash = password::hash(pw.clone()).await?;
    let user = db::users::insert(pool, email, &hash, Role::Owner)
        .await
        .context("creating owner user")?;
    tracing::info!(email = %user.email, "created initial owner account");
    Ok(())
}
