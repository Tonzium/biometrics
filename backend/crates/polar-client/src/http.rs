//! Yhteiset HTTP-apurit: virhekoodien muunnos ja JSON-purku.

use reqwest::{RequestBuilder, Response, StatusCode};
use serde::de::DeserializeOwned;

use crate::PolarError;

/// Muuntaa 429:n ja muut virhekoodit `PolarError`iksi. Onnistunut vastaus
/// palautetaan sellaisenaan.
pub(crate) async fn check(response: Response) -> Result<Response, PolarError> {
    let status = response.status();
    if status.is_success() {
        return Ok(response);
    }
    if status == StatusCode::TOO_MANY_REQUESTS {
        let reset_secs = response
            .headers()
            .get("ratelimit-reset")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.trim().parse().ok());
        return Err(PolarError::RateLimited { reset_secs });
    }
    let body = response.text().await.unwrap_or_default();
    Err(PolarError::Status {
        status: status.as_u16(),
        body: body.chars().take(500).collect(),
    })
}

pub(crate) async fn json<T: DeserializeOwned>(response: Response) -> Result<T, PolarError> {
    let bytes = response.bytes().await?;
    serde_json::from_slice(&bytes).map_err(|e| {
        let preview: String = String::from_utf8_lossy(&bytes).chars().take(200).collect();
        PolarError::Decode(format!("{e} (body: {preview})"))
    })
}

pub(crate) fn bearer(request: RequestBuilder, token: &str) -> RequestBuilder {
    request
        .bearer_auth(token)
        .header("Accept", "application/json")
}
