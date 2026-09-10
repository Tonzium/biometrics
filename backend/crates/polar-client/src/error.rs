use thiserror::Error;

#[derive(Debug, Error)]
pub enum PolarError {
    /// Verkko- tai TLS-virhe, aikakatkaisu tms.
    #[error("polar transport error: {0}")]
    Transport(#[from] reqwest::Error),

    /// HTTP 429. `reset_secs` on Polarin `RateLimit-Reset`-otsake, jos annettu.
    #[error("polar rate limit exceeded (reset in {reset_secs:?} s)")]
    RateLimited { reset_secs: Option<u64> },

    /// Muu 4xx/5xx-vastaus.
    #[error("polar returned HTTP {status}: {body}")]
    Status { status: u16, body: String },

    /// Vastaus ei ollut odotettua muotoa.
    #[error("could not decode polar response: {0}")]
    Decode(String),
}

impl PolarError {
    pub fn status(&self) -> Option<u16> {
        match self {
            Self::Status { status, .. } => Some(*status),
            Self::RateLimited { .. } => Some(429),
            _ => None,
        }
    }
}
