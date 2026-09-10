//! OAuth2 authorization code -virta Polar Flow'ta vastaan.
//!
//! 1. Käyttäjä ohjataan [`PolarClient::authorization_url`]-osoitteeseen.
//! 2. Polar palauttaa selaimen `redirect_url`:iin parametreilla `code` ja `state`.
//! 3. [`PolarClient::exchange_code`] vaihtaa koodin access tokeniin.
//!    Koodi on voimassa 10 minuuttia; token ei vanhene, ellei sitä peruta.

use serde::Deserialize;
use url::Url;

use crate::{PolarClient, PolarError, http};

#[derive(Debug, Clone, Deserialize)]
pub struct TokenResponse {
    pub access_token: String,
    #[serde(default)]
    pub token_type: String,
    #[serde(default)]
    pub expires_in: Option<u64>,
    /// Polarin käyttäjä-id. Tarvitaan `/v3/users/{id}`-reitteihin.
    pub x_user_id: u64,
}

impl PolarClient {
    /// Rakentaa osoitteen, johon selain ohjataan. `state` on satunnainen
    /// CSRF-token, joka tarkistetaan paluuohjauksessa.
    pub fn authorization_url(&self, state: &str) -> String {
        let mut url = Url::parse(&self.config.authorization_url)
            .expect("authorization_url is validated at construction");
        url.query_pairs_mut()
            .append_pair("response_type", "code")
            .append_pair("client_id", &self.config.client_id)
            .append_pair("redirect_uri", &self.config.redirect_url)
            .append_pair("scope", "accesslink.read_all")
            .append_pair("state", state);
        url.into()
    }

    pub async fn exchange_code(&self, code: &str) -> Result<TokenResponse, PolarError> {
        let response = self
            .http
            .post(&self.config.token_url)
            .basic_auth(&self.config.client_id, Some(&self.config.client_secret))
            .header("Accept", "application/json;charset=UTF-8")
            .form(&[
                ("grant_type", "authorization_code"),
                ("code", code),
                ("redirect_uri", self.config.redirect_url.as_str()),
            ])
            .send()
            .await?;
        http::json(http::check(response).await?).await
    }
}

#[cfg(test)]
mod tests {
    use wiremock::{
        Mock, MockServer, ResponseTemplate,
        matchers::{basic_auth, body_string_contains, method, path},
    };

    use super::*;
    use crate::PolarConfig;

    #[test]
    fn authorization_url_contains_all_parameters() {
        let client = PolarClient::new(PolarConfig::new(
            "cid".into(),
            "secret".into(),
            "https://example.com/cb".into(),
        ))
        .unwrap();
        let url = Url::parse(&client.authorization_url("abc123")).unwrap();
        assert_eq!(url.host_str(), Some("flow.polar.com"));
        let q: Vec<(String, String)> = url
            .query_pairs()
            .map(|(k, v)| (k.into(), v.into()))
            .collect();
        assert!(q.contains(&("response_type".into(), "code".into())));
        assert!(q.contains(&("client_id".into(), "cid".into())));
        assert!(q.contains(&("redirect_uri".into(), "https://example.com/cb".into())));
        assert!(q.contains(&("state".into(), "abc123".into())));
    }

    #[tokio::test]
    async fn exchange_code_posts_form_with_basic_auth() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v2/oauth2/token"))
            .and(basic_auth("test-client-id", "test-client-secret"))
            .and(body_string_contains("grant_type=authorization_code"))
            .and(body_string_contains("code=the-code"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "access_token": "tok-123",
                "token_type": "bearer",
                "expires_in": 473040000,
                "x_user_id": 987654
            })))
            .expect(1)
            .mount(&server)
            .await;

        let client = PolarClient::new(PolarConfig::for_base_url(&server.uri())).unwrap();
        let token = client.exchange_code("the-code").await.unwrap();
        assert_eq!(token.access_token, "tok-123");
        assert_eq!(token.x_user_id, 987654);
    }

    #[tokio::test]
    async fn exchange_code_maps_error_status() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v2/oauth2/token"))
            .respond_with(ResponseTemplate::new(401).set_body_string("invalid_client"))
            .mount(&server)
            .await;

        let client = PolarClient::new(PolarConfig::for_base_url(&server.uri())).unwrap();
        let err = client.exchange_code("x").await.unwrap_err();
        assert_eq!(err.status(), Some(401));
    }

    #[tokio::test]
    async fn rate_limit_is_reported_with_reset() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v2/oauth2/token"))
            .respond_with(ResponseTemplate::new(429).insert_header("RateLimit-Reset", "120"))
            .mount(&server)
            .await;

        let client = PolarClient::new(PolarConfig::for_base_url(&server.uri())).unwrap();
        match client.exchange_code("x").await.unwrap_err() {
            PolarError::RateLimited { reset_secs } => assert_eq!(reset_secs, Some(120)),
            other => panic!("unexpected error {other:?}"),
        }
    }
}
