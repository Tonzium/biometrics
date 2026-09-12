//! Yhtenäinen virhemuoto koko rajapinnalle.
//!
//! Jokainen reitti palauttaa `Result<T, ApiError>`. Virhe muunnetaan
//! HTTP-vastaukseksi muodossa `{ "error": { "code": "...", "message": "..." } }`.
//! Sisäiset virheet (kanta, verkko) lokitetaan täydellisinä mutta asiakkaalle
//! näytetään vain yleinen viesti, ettei toteutuksen yksityiskohtia vuoda.

use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use domain::DomainError;
use serde::Serialize;
use utoipa::ToSchema;

#[derive(Debug)]
pub enum ApiError {
    BadRequest(String),
    Unauthorized,
    Forbidden,
    NotFound(String),
    Conflict(String),
    /// Pyyntöjä tuli liikaa: joko ulkoinen palvelu (Polar) rajoitti meitä tai
    /// oma rinnakkaisuusraja täyttyi (kirjautuminen). `retry_after_secs`
    /// välitetään asiakkaalle `Retry-After`-otsakkeessa.
    TooManyRequests {
        message: String,
        retry_after_secs: Option<u64>,
    },
    /// Toiminto vaatii asetuksen, jota ei ole (esim. Polar-tunnukset).
    ServiceUnavailable(String),
    Internal(anyhow::Error),
}

/// Kaikkien virhevastausten muoto.
#[derive(Serialize, ToSchema)]
pub struct ErrorBody<'a> {
    pub error: ErrorDetail<'a>,
}

#[derive(Serialize, ToSchema)]
pub struct ErrorDetail<'a> {
    /// Koneluettava koodi, esim. `unauthorized`, `not_found`.
    pub code: &'a str,
    pub message: String,
}

impl ApiError {
    fn status_code_and_message(&self) -> (StatusCode, &'static str, String) {
        match self {
            Self::BadRequest(m) => (StatusCode::BAD_REQUEST, "bad_request", m.clone()),
            Self::Unauthorized => (
                StatusCode::UNAUTHORIZED,
                "unauthorized",
                "authentication required".into(),
            ),
            Self::Forbidden => (StatusCode::FORBIDDEN, "forbidden", "not allowed".into()),
            Self::NotFound(m) => (StatusCode::NOT_FOUND, "not_found", m.clone()),
            Self::Conflict(m) => (StatusCode::CONFLICT, "conflict", m.clone()),
            Self::TooManyRequests { message, .. } => (
                StatusCode::TOO_MANY_REQUESTS,
                "too_many_requests",
                message.clone(),
            ),
            Self::ServiceUnavailable(m) => {
                (StatusCode::SERVICE_UNAVAILABLE, "unavailable", m.clone())
            }
            Self::Internal(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "internal",
                "internal server error".into(),
            ),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        if let Self::Internal(err) = &self {
            tracing::error!(error = ?err, "internal error");
        }
        let (status, code, message) = self.status_code_and_message();
        let body = Json(ErrorBody {
            error: ErrorDetail { code, message },
        });

        let mut response = (status, body).into_response();
        if let Self::TooManyRequests {
            retry_after_secs: Some(secs),
            ..
        } = self
            && let Ok(value) = secs.to_string().parse()
        {
            response.headers_mut().insert("retry-after", value);
        }
        response
    }
}

impl From<DomainError> for ApiError {
    fn from(err: DomainError) -> Self {
        match err {
            DomainError::NotFound(m) => Self::NotFound(m),
            DomainError::Validation(m) => Self::BadRequest(m),
            DomainError::Unauthorized => Self::Unauthorized,
            DomainError::Conflict(m) => Self::Conflict(m),
        }
    }
}

impl From<sqlx::Error> for ApiError {
    fn from(err: sqlx::Error) -> Self {
        match err {
            sqlx::Error::RowNotFound => Self::NotFound("resource not found".into()),
            other => Self::Internal(other.into()),
        }
    }
}

impl From<anyhow::Error> for ApiError {
    fn from(err: anyhow::Error) -> Self {
        Self::Internal(err)
    }
}

/// Lyhenne reittien paluutyypille.
pub type ApiResult<T> = Result<T, ApiError>;
