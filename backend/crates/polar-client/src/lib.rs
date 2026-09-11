//! Polar AccessLink API v3 -asiakas.
//!
//! Dokumentaatio: https://www.polar.com/accesslink-api/
//! OpenAPI-kuvaus (swagger): https://www.polar.com/accesslink-api/#polar-accesslink-api
//!
//! Rakenne:
//! - [`oauth`]  valtuutus-URL ja koodin vaihto access tokeniin
//! - [`users`]  käyttäjän rekisteröinti/poisto AccessLinkissä
//! - [`data`]   datareitit: exercises, sleep, nightly-recharge, activities, physical-info, cardio-load
//! - [`models`] vastausrakenteet, [`duration`] ISO 8601 -kestot
//!
//! Kaikki URL:t ovat [`PolarConfig`]:ssa, jotta testit voivat osoittaa
//! ne mock-palvelimeen.

pub mod data;
pub mod duration;
mod error;
mod http;
pub mod models;
pub mod oauth;
pub mod users;

use std::{sync::Arc, time::Duration};

pub use error::PolarError;

/// Käyttäjän ohjaus valtuutukseen (selain).
pub const AUTHORIZATION_URL: &str = "https://flow.polar.com/oauth2/authorization";
/// Valtuutuskoodin vaihto access tokeniin (Basic auth client id/secret).
pub const TOKEN_URL: &str = "https://polarremote.com/v2/oauth2/token";
/// REST-rajapinnan juuri.
pub const API_BASE_URL: &str = "https://www.polaraccesslink.com/v3";

#[derive(Debug, Clone)]
pub struct PolarConfig {
    pub client_id: String,
    pub client_secret: String,
    pub redirect_url: String,
    pub authorization_url: String,
    pub token_url: String,
    pub api_base_url: String,
}

impl PolarConfig {
    /// Tuotantoasetukset Polarin virallisilla osoitteilla.
    pub fn new(client_id: String, client_secret: String, redirect_url: String) -> Self {
        Self {
            client_id,
            client_secret,
            redirect_url,
            authorization_url: AUTHORIZATION_URL.to_owned(),
            token_url: TOKEN_URL.to_owned(),
            api_base_url: API_BASE_URL.to_owned(),
        }
    }

    /// Asetukset, joissa kaikki osoitteet viittaavat annettuun juureen
    /// (testien mock-palvelin).
    pub fn for_base_url(base: &str) -> Self {
        let base = base.trim_end_matches('/');
        Self {
            client_id: "test-client-id".into(),
            client_secret: "test-client-secret".into(),
            redirect_url: "http://localhost/api/polar/callback".into(),
            authorization_url: format!("{base}/oauth2/authorization"),
            token_url: format!("{base}/v2/oauth2/token"),
            api_base_url: format!("{base}/v3"),
        }
    }
}

#[derive(Clone)]
pub struct PolarClient {
    http: reqwest::Client,
    config: Arc<PolarConfig>,
}

impl PolarClient {
    pub fn new(config: PolarConfig) -> Result<Self, PolarError> {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .user_agent(concat!("polar-data-hub/", env!("CARGO_PKG_VERSION")))
            .build()?;
        Ok(Self {
            http,
            config: Arc::new(config),
        })
    }

    pub fn config(&self) -> &PolarConfig {
        &self.config
    }

    fn api_url(&self, path: &str) -> String {
        format!("{}{}", self.config.api_base_url, path)
    }
}
