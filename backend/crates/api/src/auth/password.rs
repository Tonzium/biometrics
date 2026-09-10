//! Salasanojen tiivistys argon2id-algoritmilla (PHC-merkkijono).
//!
//! Tiivistys on tarkoituksella hidas (~100 ms), joten se ajetaan
//! `spawn_blocking`-säikeessä, ettei se tuki tokion async-työntekijöitä.

use anyhow::{Context, anyhow};
use argon2::{
    Argon2,
    password_hash::{PasswordHasher, PasswordVerifier, phc::PasswordHash},
};

/// Kiinteä tiiviste, jota vasten verrataan kun käyttäjää ei löydy.
/// Näin kirjautumisen kesto ei paljasta, onko sähköposti olemassa.
const DUMMY_HASH: &str = "$argon2id$v=19$m=19456,t=2,p=1$YWFhYWFhYWFhYWFhYWFhYQ$Kq3z5Z6c3a8b1Wc8G0Q7kJz3s1Zt1uYd2i9Yd6wNw3Y";

pub async fn hash(password: String) -> anyhow::Result<String> {
    tokio::task::spawn_blocking(move || {
        Argon2::default()
            .hash_password(password.as_bytes())
            .map(|h| h.to_string())
            .map_err(|e| anyhow!("hashing password: {e}"))
    })
    .await
    .context("hash task panicked")?
}

/// Palauttaa `true`, jos salasana täsmää. Virheellinen PHC-merkkijono
/// tulkitaan epäonnistuneeksi kirjautumiseksi, ei palvelinvirheeksi.
pub async fn verify(password: String, phc_hash: Option<String>) -> anyhow::Result<bool> {
    tokio::task::spawn_blocking(move || {
        let stored = phc_hash.as_deref().unwrap_or(DUMMY_HASH);
        let Ok(parsed) = PasswordHash::new(stored) else {
            return false;
        };
        let matches = Argon2::default()
            .verify_password(password.as_bytes(), &parsed)
            .is_ok();
        // Jos tiiviste oli valetiiviste, tulos on aina epätosi.
        matches && phc_hash.is_some()
    })
    .await
    .context("verify task panicked")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn hash_and_verify_roundtrip() {
        let h = hash("hunter42".into()).await.unwrap();
        assert!(h.starts_with("$argon2id$"));
        assert!(verify("hunter42".into(), Some(h.clone())).await.unwrap());
        assert!(!verify("wrong".into(), Some(h)).await.unwrap());
    }

    #[tokio::test]
    async fn missing_user_never_verifies() {
        assert!(!verify("anything".into(), None).await.unwrap());
    }

    #[tokio::test]
    async fn garbage_hash_is_not_an_error() {
        assert!(!verify("x".into(), Some("not-a-hash".into())).await.unwrap());
    }
}
