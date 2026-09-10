//! Polar AccessLink API v3 -asiakas.
//!
//! Vaihe 3 täydentää tämän OAuth2-virralla ja datareiteillä.
//! Dokumentaatio: https://www.polar.com/accesslink-api/

/// Käyttäjän ohjaus valtuutukseen (selain).
pub const AUTHORIZATION_URL: &str = "https://flow.polar.com/oauth2/authorization";
/// Valtuutuskoodin vaihto access tokeniin (Basic auth client id/secret).
pub const TOKEN_URL: &str = "https://polarremote.com/v2/oauth2/token";
/// REST-rajapinnan juuri.
pub const API_BASE_URL: &str = "https://www.polaraccesslink.com/v3";
