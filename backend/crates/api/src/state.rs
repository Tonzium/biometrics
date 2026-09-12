use std::sync::Arc;

use polar_client::PolarClient;
use sqlx::PgPool;
use tokio::sync::{Mutex, Semaphore};

use crate::{auth::LOGIN_MAX_CONCURRENT, config::Config, crypto::TokenCipher};

/// Jaettu tila, joka annetaan jokaiselle reitille (`State<AppState>`).
/// Kaikki kentät ovat halpoja kloonata (`Arc` tai sisäisesti `Arc`).
#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    pub pool: PgPool,
    pub cipher: Arc<TokenCipher>,
    /// `None`, jos Polar-tunnuksia ei ole asetettu.
    pub polar: Option<PolarClient>,
    /// Varmistaa, että vain yksi synkronointi on käynnissä kerrallaan.
    pub sync_lock: Arc<Mutex<()>>,
    /// Rajaa rinnakkaiset salasanatarkistukset (`LOGIN_MAX_CONCURRENT`). Kun
    /// permittejä ei ole vapaana, kirjautuminen hylätään heti 429:llä eikä
    /// uutta argon2-laskentaa käynnistetä lainkaan.
    pub login_limit: Arc<Semaphore>,
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
            sync_lock: Arc::new(Mutex::new(())),
            login_limit: Arc::new(Semaphore::new(LOGIN_MAX_CONCURRENT)),
        })
    }
}
