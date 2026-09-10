use std::sync::Arc;

use polar_client::PolarClient;
use sqlx::PgPool;

use crate::{config::Config, crypto::TokenCipher};

/// Jaettu tila, joka annetaan jokaiselle reitille (`State<AppState>`).
/// Kaikki kentät ovat halpoja kloonata (`Arc` tai sisäisesti `Arc`).
#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    pub pool: PgPool,
    pub cipher: Arc<TokenCipher>,
    /// `None`, jos Polar-tunnuksia ei ole asetettu.
    pub polar: Option<PolarClient>,
}

impl AppState {
    pub fn new(config: Config, pool: PgPool) -> anyhow::Result<Self> {
        let cipher = Arc::new(TokenCipher::from_key_bytes(&config.encryption_key));
        let polar = config.polar.clone().map(PolarClient::new).transpose()?;
        Ok(Self {
            config: Arc::new(config),
            pool,
            cipher,
            polar,
        })
    }
}
