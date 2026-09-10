//! Polar Data Hub -backend, käynnistin.
//!
//! Lue asetukset ympäristöstä, avaa kantayhteys, aja migraatiot,
//! rakenna reititin ja kuuntele `BIND_ADDR`-osoitteessa.

use anyhow::Context;
use api::{AppState, Config, MIGRATOR};
use sqlx::postgres::PgPoolOptions;
use tokio::net::TcpListener;
use tracing_subscriber::{EnvFilter, fmt};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // .env on vain kehitystä varten; tuotannossa compose asettaa muuttujat.
    let _ = dotenvy::from_filename("../deploy/.env");

    fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .init();

    let config = Config::from_env().context("configuration")?;

    let pool = PgPoolOptions::new()
        .max_connections(10)
        .connect(&config.database_url)
        .await
        .context("connecting to PostgreSQL")?;

    MIGRATOR.run(&pool).await.context("running migrations")?;
    tracing::info!("migrations applied");

    api::seed::ensure_owner(&pool, &config)
        .await
        .context("seeding owner account")?;

    let state = AppState::new(config.clone(), pool).context("building app state")?;
    api::sync::scheduler::spawn(state.clone());
    let app = api::app(state);

    let listener = TcpListener::bind(&config.bind_addr)
        .await
        .with_context(|| format!("binding {}", config.bind_addr))?;
    tracing::info!(addr = %config.bind_addr, "listening");

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .context("server error")?;

    Ok(())
}

async fn shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
    tracing::info!("shutdown signal received");
}
