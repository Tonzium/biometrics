//! Polar Data Hub -backend kirjastona.
//!
//! `main.rs` on ohut käynnistin; kaikki logiikka on täällä, jotta
//! integraatiotestit (`tests/`) voivat rakentaa saman reitittimen
//! testikantaa vasten ilman verkkoporttia.

pub mod config;
#[allow(dead_code)] // otetaan käyttöön auth-reiteissä (vaihe 2)
pub mod error;
pub mod routes;
pub mod state;

pub use config::Config;
pub use state::AppState;

/// Rakentaa koko sovelluksen reitittimen annetulla tilalla.
pub fn app(state: AppState) -> axum::Router {
    routes::router(state)
}

/// Sovelluksen migraatiot; sama joukko ajetaan käynnistyksessä ja testeissä.
pub static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("../../migrations");
