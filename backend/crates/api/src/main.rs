//! Polar Data Hub -backend.
//!
//! Käynnistys: lue asetukset ympäristöstä, avaa kantayhteys, aja migraatiot,
//! rakenna reititin ja kuuntele `BIND_ADDR`-osoitteessa.

mod config;
mod routes;
mod state;

use anyhow::Context;
use sqlx::postgres::PgPoolOptions;
use tokio::net::TcpListener;
use tracing_subscriber::{EnvFilter, fmt};

use crate::config::Config;
use crate::state::AppState;

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

    sqlx::migrate!("../../migrations")
        .run(&pool)
        .await
        .context("running migrations")?;
    tracing::info!("migrations applied");

    let state = AppState::new(config.clone(), pool);
    let app = routes::router(state);

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
