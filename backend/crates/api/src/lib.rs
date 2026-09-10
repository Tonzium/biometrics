//! Polar Data Hub -backend kirjastona.
//!
//! `main.rs` on ohut käynnistin; kaikki logiikka on täällä, jotta
//! integraatiotestit (`tests/`) voivat rakentaa saman reitittimen
//! testikantaa vasten ilman verkkoporttia.

pub mod auth;
pub mod config;
pub mod crypto;
pub mod db;
pub mod error;
pub mod routes;
pub mod seed;
pub mod state;

pub use config::Config;
pub use state::AppState;

/// Rakentaa koko sovelluksen reitittimen annetulla tilalla.
pub fn app(state: AppState) -> axum::Router {
    routes::router(state)
}

/// Sovelluksen migraatiot; sama joukko ajetaan käynnistyksessä ja testeissä.
pub static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("../../migrations");
