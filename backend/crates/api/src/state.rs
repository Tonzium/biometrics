use sqlx::PgPool;

use crate::config::Config;

/// Jaettu tila, joka annetaan jokaiselle reitille (`State<AppState>`).
/// `PgPool` on sisäisesti `Arc`, joten kloonaus on halpaa.
#[derive(Clone)]
pub struct AppState {
    #[allow(dead_code)] // otetaan käyttöön auth- ja sync-vaiheissa
    pub config: Config,
    pub pool: PgPool,
}

impl AppState {
    pub fn new(config: Config, pool: PgPool) -> Self {
        Self { config, pool }
    }
}
