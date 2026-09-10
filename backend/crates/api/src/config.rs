use std::env;

use anyhow::{Context, bail};

/// Kaikki ympäristöstä luettavat asetukset. Puuttuva pakollinen arvo
/// kaataa käynnistyksen heti selkeällä virheellä.
#[derive(Debug, Clone)]
pub struct Config {
    pub database_url: String,
    pub bind_addr: String,
}

impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        let database_url = required("DATABASE_URL")?;
        let bind_addr = env::var("BIND_ADDR").unwrap_or_else(|_| "127.0.0.1:8787".to_owned());

        Ok(Self {
            database_url,
            bind_addr,
        })
    }
}

fn required(key: &str) -> anyhow::Result<String> {
    let value = env::var(key).with_context(|| format!("{key} is not set"))?;
    if value.trim().is_empty() {
        bail!("{key} is empty");
    }
    Ok(value)
}
