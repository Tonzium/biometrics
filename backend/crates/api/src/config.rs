use std::env;

use anyhow::{Context, bail};

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

        let admin_email = env::var("ADMIN_EMAIL")
            .ok()
            .map(|e| e.trim().to_lowercase())
            .filter(|e| !e.is_empty());
        let admin_password = env::var("ADMIN_PASSWORD").ok().filter(|p| !p.is_empty());

        Ok(Self {
            database_url,
            bind_addr,
            jwt_secret,
            session_hours,
            cookie_secure,
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

fn optional_parsed<T: std::str::FromStr>(key: &str, default: T) -> anyhow::Result<T>
where
    T::Err: std::fmt::Display,
{
    match env::var(key) {
        Ok(raw) if !raw.trim().is_empty() => raw
            .trim()
            .parse()
            .map_err(|e| anyhow::anyhow!("{key}={raw:?} is invalid: {e}")),
        _ => Ok(default),
    }
}
