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

/// Montako salasanatarkistusta saa olla käynnissä yhtä aikaa.
///
/// Yksi argon2id-vertailu (m=19 MiB, t=2, p=1) maksaa release-buildissa noin
/// 10 ms CPU-aikaa mitattuna kehityskoneella – tuotannon hitaammalla ytimellä
/// enemmän – ja varaa laskennan ajaksi 19 MiB muistia. Muisti on tässä se
/// kriittinen resurssi: vertailu ajetaan `spawn_blocking`illa tokion
/// blokkaavassa säiepoolissa, johon mahtuu oletuksena 512 tehtävää, joten
/// rajaton kirjautumistulva yrittäisi varata luokkaa 9 GiB ja tappaisi kontin
/// muistin loppumiseen ennen kuin yksikään kirjautuminen valmistuu. Kaksi
/// permittiä rajaa tämän noin 38 MiB:iin ja kahteen ytimeen, ja sietää silti
/// käyttäjän oman tuplaklikkauksen. Ylimenevät pyynnöt hylätään heti 429:llä;
/// ne eivät jonota, koska jonottaminen vain siirtäisi saman kuorman
/// myöhemmäksi.
///
/// Raja on tarkoituksella vakio eikä ympäristömuuttuja: se on turvaraja, ei
/// säätönuppi. Jos lukua nostetaan ydinten määrään tai yli, suoja katoaa.
pub const LOGIN_MAX_CONCURRENT: usize = 2;

/// `Retry-After`-otsakkeen arvo sekunteina, kun kirjautuminen hylätään ruuhkan
/// takia. Yksi tarkistus kestää ~0,2 s, joten sekunnin kuluttua permit on jo
/// vapaa.
pub const LOGIN_RETRY_AFTER_SECS: u64 = 1;

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
