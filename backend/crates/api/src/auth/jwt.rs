//! JWT-istuntotokenit (HS256).

use anyhow::Context;
use chrono::{Duration, Utc};
use domain::{Role, User};
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::config::Config;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    /// Käyttäjän id.
    pub sub: Uuid,
    pub email: String,
    pub role: Role,
    /// Istuntoversio myöntöhetkellä. `CurrentUser` vertaa tätä kannan arvoon,
    /// joten version kasvattaminen mitätöi tämän tokenin.
    pub ver: i32,
    /// Myöntöhetki (unix-sekunnit).
    pub iat: i64,
    /// Vanhenemishetki (unix-sekunnit).
    pub exp: i64,
}

/// `token_version` tulee kantariviltä, ei `User`-rakenteesta: `User`
/// sarjallistetaan rajapinnasta ulos, eikä istuntoversio kuulu sinne.
pub fn issue(config: &Config, user: &User, token_version: i32) -> anyhow::Result<String> {
    let now = Utc::now();
    let claims = Claims {
        sub: user.id,
        email: user.email.clone(),
        role: user.role,
        ver: token_version,
        iat: now.timestamp(),
        exp: (now + Duration::hours(config.session_hours)).timestamp(),
    };
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(config.jwt_secret.as_bytes()),
    )
    .context("signing session token")
}

/// Tarkistaa allekirjoituksen ja vanhenemisen. Virhe tarkoittaa aina
/// "ei kirjautunut", syytä ei kerrota asiakkaalle.
pub fn verify(config: &Config, token: &str) -> anyhow::Result<Claims> {
    let mut validation = Validation::new(Algorithm::HS256);
    validation.set_required_spec_claims(&["exp"]);
    validation.leeway = 30;

    let data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(config.jwt_secret.as_bytes()),
        &validation,
    )
    .context("invalid session token")?;
    Ok(data.claims)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn user() -> User {
        User {
            id: Uuid::now_v7(),
            email: "a@example.com".into(),
            role: Role::Owner,
            created_at: Utc::now(),
        }
    }

    #[test]
    fn issue_and_verify_roundtrip() {
        let config = Config::for_tests();
        let u = user();
        let token = issue(&config, &u, 7).unwrap();
        let claims = verify(&config, &token).unwrap();
        assert_eq!(claims.sub, u.id);
        assert_eq!(claims.email, u.email);
        assert_eq!(claims.role, Role::Owner);
        assert_eq!(claims.ver, 7);
        assert!(claims.exp > claims.iat);
    }

    #[test]
    fn wrong_secret_is_rejected() {
        let config = Config::for_tests();
        let token = issue(&config, &user(), 1).unwrap();
        let other = Config {
            jwt_secret: "another-secret-another-secret-another-secret".into(),
            ..Config::for_tests()
        };
        assert!(verify(&other, &token).is_err());
    }

    #[test]
    fn tampered_token_is_rejected() {
        let config = Config::for_tests();
        let mut token = issue(&config, &user(), 1).unwrap();
        token.push('x');
        assert!(verify(&config, &token).is_err());
    }
}
