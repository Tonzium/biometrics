//! Polar-tokenin salaus levossa (AES-256-GCM).
//!
//! Tallennusmuoto: `nonce (12 tavua) || ciphertext+tag`. Avain tulee
//! `APP_ENCRYPTION_KEY`-muuttujasta base64-koodattuna (32 tavua).
//! Jos avain vaihtuu, vanhat tokenit eivät enää aukea ja Polar-tili on
//! yhdistettävä uudelleen.

use aes_gcm::{
    Aes256Gcm, Key, Nonce,
    aead::{Aead, AeadCore, Generate, KeyInit},
};
use anyhow::{Context, anyhow, bail};
use base64::{Engine, engine::general_purpose::STANDARD};

const NONCE_LEN: usize = 12;

pub struct TokenCipher {
    cipher: Aes256Gcm,
}

impl TokenCipher {
    pub fn from_key_bytes(key: &[u8; 32]) -> Self {
        Self {
            cipher: Aes256Gcm::new(&Key::<Aes256Gcm>::from(*key)),
        }
    }

    /// Purkaa base64-koodatun 32-tavuisen avaimen.
    pub fn parse_key(encoded: &str) -> anyhow::Result<[u8; 32]> {
        let bytes = STANDARD
            .decode(encoded.trim())
            .context("APP_ENCRYPTION_KEY is not valid base64")?;
        let key: [u8; 32] = bytes
            .try_into()
            .map_err(|_| anyhow!("APP_ENCRYPTION_KEY must decode to exactly 32 bytes"))?;
        Ok(key)
    }

    pub fn encrypt(&self, plaintext: &str) -> anyhow::Result<Vec<u8>> {
        let nonce = Nonce::<<Aes256Gcm as AeadCore>::NonceSize>::generate();
        let ciphertext = self
            .cipher
            .encrypt(&nonce, plaintext.as_bytes())
            .map_err(|_| anyhow!("encryption failed"))?;
        let mut out = Vec::with_capacity(NONCE_LEN + ciphertext.len());
        out.extend_from_slice(&nonce);
        out.extend_from_slice(&ciphertext);
        Ok(out)
    }

    pub fn decrypt(&self, data: &[u8]) -> anyhow::Result<String> {
        if data.len() < NONCE_LEN + 16 {
            bail!("ciphertext too short");
        }
        let (nonce, ciphertext) = data.split_at(NONCE_LEN);
        let nonce = Nonce::<<Aes256Gcm as AeadCore>::NonceSize>::try_from(nonce)
            .map_err(|_| anyhow!("invalid nonce length"))?;
        let plaintext = self
            .cipher
            .decrypt(&nonce, ciphertext)
            .map_err(|_| anyhow!("decryption failed (wrong key or corrupted data)"))?;
        String::from_utf8(plaintext).context("decrypted token is not UTF-8")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_and_unique_nonces() {
        let c = TokenCipher::from_key_bytes(&[7u8; 32]);
        let a = c.encrypt("secret-token").unwrap();
        let b = c.encrypt("secret-token").unwrap();
        assert_ne!(a, b, "nonce must differ per encryption");
        assert_eq!(c.decrypt(&a).unwrap(), "secret-token");
        assert_eq!(c.decrypt(&b).unwrap(), "secret-token");
    }

    #[test]
    fn wrong_key_fails() {
        let a = TokenCipher::from_key_bytes(&[1u8; 32]);
        let b = TokenCipher::from_key_bytes(&[2u8; 32]);
        let ct = a.encrypt("x").unwrap();
        assert!(b.decrypt(&ct).is_err());
    }

    #[test]
    fn tampering_is_detected() {
        let c = TokenCipher::from_key_bytes(&[3u8; 32]);
        let mut ct = c.encrypt("token").unwrap();
        let last = ct.len() - 1;
        ct[last] ^= 0x01;
        assert!(c.decrypt(&ct).is_err());
    }

    #[test]
    fn parse_key_validates_length() {
        assert!(TokenCipher::parse_key(&STANDARD.encode([0u8; 32])).is_ok());
        assert!(TokenCipher::parse_key(&STANDARD.encode([0u8; 16])).is_err());
        assert!(TokenCipher::parse_key("not base64!").is_err());
    }
}
