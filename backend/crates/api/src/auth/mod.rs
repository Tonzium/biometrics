//! Autentikointi: salasanatiivisteet, JWT-istunnot ja `CurrentUser`-ekstraktori.
//!
//! Istunto kulkee httpOnly-cookiessa, jota selaimen JavaScript ei näe.
//! `SameSite=Lax` estää cookien lähettämisen cross-site POST -pyynnöissä
//! (CSRF-suoja) mutta sallii sen ylätason GET-navigoinnissa, jota Polarin
//! OAuth-paluuohjaus tarvitsee.

pub mod extract;
pub mod jwt;
pub mod password;

pub use extract::{CurrentUser, ReadAccess};

use axum_extra::extract::cookie::{Cookie, SameSite};
use time::Duration;

use crate::config::Config;

/// Istuntocookien nimi.
pub const SESSION_COOKIE: &str = "pdh_session";

/// Rakentaa istuntocookien annetulle tokenille.
pub fn session_cookie(config: &Config, token: String) -> Cookie<'static> {
    Cookie::build((SESSION_COOKIE, token))
        .path("/")
        .http_only(true)
        .secure(config.cookie_secure)
        .same_site(SameSite::Lax)
        .max_age(Duration::hours(config.session_hours))
        .build()
}

/// Cookie, joka poistaa istunnon selaimesta.
pub fn removal_cookie(config: &Config) -> Cookie<'static> {
    Cookie::build((SESSION_COOKIE, ""))
        .path("/")
        .http_only(true)
        .secure(config.cookie_secure)
        .same_site(SameSite::Lax)
        .max_age(Duration::ZERO)
        .build()
}
