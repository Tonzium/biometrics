use std::env;

use anyhow::{Context, bail};
use polar_client::PolarConfig;

/// Kaikki ympäristöstä luettavat asetukset. Puuttuva pakollinen arvo
/// kaataa käynnistyksen heti selkeällä virheellä.
#[derive(Debug, Clone)]
pub struct Config {
    pub database_url: String,
    pub bind_addr: String,

    /// JWT-allekirjoitusavain (HS256). Vähintään 32 merkkiä.
    pub jwt_secret: String,
    /// Istunnon kesto tunteina.
    pub session_hours: i64,
    /// Cookie `Secure`-lippu. Kehityksessä http://localhost vaatii `false`.
    pub cookie_secure: bool,

    /// AES-256-avain Polar-tokenin salaamiseen levossa.
    pub encryption_key: [u8; 32],

    /// Saako dataa lukea ilman kirjautumista (näyteikkuna). `false` = kaikki
    /// reitit vaativat istunnon.
    pub public_read: bool,
    /// Näytetäänkö paino ja pituus myös kirjautumattomille. Oletus `false`:
    /// julkisessa näyteikkunassa ne piilotetaan.
    pub public_body_metrics: bool,

    /// Ajastetun synkronoinnin väli tunteina; 0 = vain manuaalinen.
    pub sync_interval_hours: u64,

    /// Polar AccessLink -asiakkaan tunnukset. `None`, jos niitä ei ole
    /// asetettu: palvelin käynnistyy, mutta Polar-yhdistäminen palauttaa 503.
    pub polar: Option<PolarConfig>,

    /// Ensimmäisellä käynnistyksellä luotava omistajakäyttäjä.
    /// Jos kannassa on jo käyttäjiä, näitä ei käytetä.
    pub admin_email: Option<String>,
    pub admin_password: Option<String>,
}

impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        let database_url = required("DATABASE_URL")?;
        let bind_addr = env::var("BIND_ADDR").unwrap_or_else(|_| "127.0.0.1:8787".to_owned());

        let jwt_secret = required("JWT_SECRET")?;
        if jwt_secret.len() < 32 {
            bail!("JWT_SECRET must be at least 32 characters");
        }
        let session_hours = optional_parsed("SESSION_HOURS", 168)?;
        if session_hours <= 0 {
            bail!("SESSION_HOURS must be positive");
        }
        let cookie_secure = optional_parsed("COOKIE_SECURE", true)?;

        let encryption_key =
            crate::crypto::TokenCipher::parse_key(&required("APP_ENCRYPTION_KEY")?)?;

        let polar = match (
            optional("POLAR_CLIENT_ID"),
            optional("POLAR_CLIENT_SECRET"),
            optional("POLAR_REDIRECT_URL"),
        ) {
            (Some(id), Some(secret), Some(redirect)) => {
                Some(PolarConfig::new(id, secret, redirect))
            }
            (None, None, _) => {
                tracing::warn!("POLAR_CLIENT_ID/SECRET not set; Polar linking is disabled");
                None
            }
            _ => {
                bail!("POLAR_CLIENT_ID, POLAR_CLIENT_SECRET and POLAR_REDIRECT_URL must all be set")
            }
        };

        let public_read = optional_parsed("PUBLIC_READ", true)?;
        let public_body_metrics = optional_parsed("PUBLIC_BODY_METRICS", false)?;
        let sync_interval_hours = optional_parsed("SYNC_INTERVAL_HOURS", 6)?;

        let admin_email = optional("ADMIN_EMAIL").map(|e| e.to_lowercase());
        let admin_password = optional("ADMIN_PASSWORD");

        Ok(Self {
            database_url,
            bind_addr,
            jwt_secret,
            session_hours,
            cookie_secure,
            encryption_key,
            public_read,
            public_body_metrics,
            sync_interval_hours,
            polar,
            admin_email,
            admin_password,
        })
    }

    /// Asetukset testejä varten: ei lue ympäristöä.
    pub fn for_tests() -> Self {
        Self {
            database_url: String::new(),
            bind_addr: "127.0.0.1:0".into(),
            jwt_secret: "test-secret-test-secret-test-secret-1234".into(),
            session_hours: 1,
            cookie_secure: false,
            encryption_key: [7u8; 32],
            public_read: true,
            public_body_metrics: false,
            sync_interval_hours: 0,
            polar: None,
            admin_email: None,
            admin_password: None,
        }
    }
}

fn required(key: &str) -> anyhow::Result<String> {
    let value = env::var(key).with_context(|| format!("{key} is not set"))?;
    if value.trim().is_empty() {
        bail!("{key} is empty");
    }
    Ok(value)
}

fn optional(key: &str) -> Option<String> {
    env::var(key)
        .ok()
        .map(|v| v.trim().to_owned())
        .filter(|v| !v.is_empty())
}

fn optional_parsed<T: std::str::FromStr>(key: &str, default: T) -> anyhow::Result<T>
where
    T::Err: std::fmt::Display,
{
    match optional(key) {
        Some(raw) => raw
            .parse()
            .map_err(|e| anyhow::anyhow!("{key}={raw:?} is invalid: {e}")),
        None => Ok(default),
    }
}
