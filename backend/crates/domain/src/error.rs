use thiserror::Error;

/// Sovellustason virheet. `api`-crate muuntaa nämä HTTP-vastauksiksi.
#[derive(Debug, Error)]
pub enum DomainError {
    #[error("not found: {0}")]
    NotFound(String),

    #[error("validation failed: {0}")]
    Validation(String),

    #[error("unauthorized")]
    Unauthorized,

    #[error("conflict: {0}")]
    Conflict(String),
}
