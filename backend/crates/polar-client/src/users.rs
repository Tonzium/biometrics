//! Käyttäjän rekisteröinti AccessLinkiin.
//!
//! Ennen kuin käyttäjän dataa voi hakea, hänet on rekisteröitävä
//! `POST /v3/users` -kutsulla. Rekisteröinti on kertaluonteinen; toinen
//! yritys palauttaa 409 Conflict, joka on normaali tilanne.

use serde::{Deserialize, Serialize};

use crate::{PolarClient, PolarError, http};

/// Polarin palauttama käyttäjäolio (kebab-case-kentät).
#[derive(Debug, Clone, Deserialize)]
pub struct PolarUser {
    #[serde(rename = "polar-user-id")]
    pub polar_user_id: u64,
    #[serde(rename = "member-id")]
    pub member_id: String,
    #[serde(rename = "registration-date")]
    pub registration_date: String,
    #[serde(rename = "first-name", default)]
    pub first_name: Option<String>,
    #[serde(rename = "last-name", default)]
    pub last_name: Option<String>,
    #[serde(default)]
    pub birthdate: Option<String>,
    #[serde(default)]
    pub gender: Option<String>,
    #[serde(default)]
    pub weight: Option<f64>,
    #[serde(default)]
    pub height: Option<f64>,
}

#[derive(Debug)]
pub enum RegisterOutcome {
    Registered(PolarUser),
    AlreadyRegistered,
}

#[derive(Serialize)]
struct RegisterBody<'a> {
    #[serde(rename = "member-id")]
    member_id: &'a str,
}

impl PolarClient {
    pub async fn register_user(
        &self,
        access_token: &str,
        member_id: &str,
    ) -> Result<RegisterOutcome, PolarError> {
        let request = http::bearer(self.http.post(self.api_url("/users")), access_token)
            .json(&RegisterBody { member_id });
        let response = request.send().await?;
        if response.status().as_u16() == 409 {
            return Ok(RegisterOutcome::AlreadyRegistered);
        }
        let user = http::json(http::check(response).await?).await?;
        Ok(RegisterOutcome::Registered(user))
    }

    pub async fn get_user(
        &self,
        access_token: &str,
        user_id: u64,
    ) -> Result<PolarUser, PolarError> {
        let request = http::bearer(
            self.http.get(self.api_url(&format!("/users/{user_id}"))),
            access_token,
        );
        http::json(http::check(request.send().await?).await?).await
    }

    /// Poistaa rekisteröinnin. 404 tulkitaan "jo poistettu".
    pub async fn delete_user(&self, access_token: &str, user_id: u64) -> Result<(), PolarError> {
        let request = http::bearer(
            self.http.delete(self.api_url(&format!("/users/{user_id}"))),
            access_token,
        );
        let response = request.send().await?;
        if response.status().as_u16() == 404 {
            return Ok(());
        }
        http::check(response).await.map(|_| ())
    }
}

#[cfg(test)]
mod tests {
    use wiremock::{
        Mock, MockServer, ResponseTemplate,
        matchers::{bearer_token, body_json, method, path},
    };

    use super::*;
    use crate::PolarConfig;

    async fn client(server: &MockServer) -> PolarClient {
        PolarClient::new(PolarConfig::for_base_url(&server.uri())).unwrap()
    }

    #[tokio::test]
    async fn register_returns_user_on_success() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v3/users"))
            .and(bearer_token("tok"))
            .and(body_json(serde_json::json!({ "member-id": "member-1" })))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "polar-user-id": 42,
                "member-id": "member-1",
                "registration-date": "2026-09-10T12:00:00.000Z",
                "first-name": "Toni",
                "last-name": "K",
                "birthdate": "1990-01-01",
                "gender": "MALE",
                "weight": 80.0,
                "height": 180.0
            })))
            .mount(&server)
            .await;

        match client(&server)
            .await
            .register_user("tok", "member-1")
            .await
            .unwrap()
        {
            RegisterOutcome::Registered(u) => {
                assert_eq!(u.polar_user_id, 42);
                assert_eq!(u.first_name.as_deref(), Some("Toni"));
            }
            other => panic!("unexpected {other:?}"),
        }
    }

    #[tokio::test]
    async fn register_conflict_is_not_an_error() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v3/users"))
            .respond_with(ResponseTemplate::new(409))
            .mount(&server)
            .await;

        let outcome = client(&server)
            .await
            .register_user("tok", "m")
            .await
            .unwrap();
        assert!(matches!(outcome, RegisterOutcome::AlreadyRegistered));
    }

    #[tokio::test]
    async fn delete_user_accepts_204_and_404() {
        let server = MockServer::start().await;
        Mock::given(method("DELETE"))
            .and(path("/v3/users/1"))
            .respond_with(ResponseTemplate::new(204))
            .mount(&server)
            .await;
        Mock::given(method("DELETE"))
            .and(path("/v3/users/2"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&server)
            .await;
        Mock::given(method("DELETE"))
            .and(path("/v3/users/3"))
            .respond_with(ResponseTemplate::new(500))
            .mount(&server)
            .await;

        let c = client(&server).await;
        c.delete_user("tok", 1).await.unwrap();
        c.delete_user("tok", 2).await.unwrap();
        assert_eq!(
            c.delete_user("tok", 3).await.unwrap_err().status(),
            Some(500)
        );
    }
}
