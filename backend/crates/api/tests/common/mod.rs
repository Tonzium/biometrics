//! Integraatiotestien apurit.
//!
//! `#[sqlx::test]` luo jokaiselle testille oman väliaikaisen tietokannan
//! `DATABASE_URL`-palvelimelle, ajaa migraatiot ja pudottaa kannan lopuksi.

#![allow(dead_code)]

use api::{AppState, Config};
use axum::{
    Router,
    body::{Body, Bytes},
    http::{Request, Response, header},
};
use http_body_util::BodyExt;
use sqlx::PgPool;
use tower::ServiceExt;

pub fn test_app(pool: PgPool) -> Router {
    test_app_with(Config::for_tests(), pool)
}

pub fn test_app_with(config: Config, pool: PgPool) -> Router {
    test_app_and_state_with(config, pool).0
}

/// Kuten [`test_app`], mutta palauttaa myös `AppState`-kopion. Tarvitaan
/// testeissä, jotka koskettavat tilan kenttiä suoraan (esim. kirjautumisen
/// rinnakkaisuusrajoitinta `login_limit`). Reititin ja palautettu tila jakavat
/// samat `Arc`:t, joten testin varaama permit on sama permit, jota käsittelijä
/// yrittää varata.
pub fn test_app_and_state(pool: PgPool) -> (Router, AppState) {
    test_app_and_state_with(Config::for_tests(), pool)
}

pub fn test_app_and_state_with(config: Config, pool: PgPool) -> (Router, AppState) {
    let state = AppState::new(config, pool).expect("test state");
    (api::app(state.clone()), state)
}

/// Lähettää pyynnön reitittimelle ilman verkkoa ja palauttaa vastauksen.
pub async fn send(app: &Router, request: Request<Body>) -> Response<Body> {
    app.clone()
        .oneshot(request)
        .await
        .expect("router should always produce a response")
}

pub async fn get(app: &Router, uri: &str) -> Response<Body> {
    send(app, Request::get(uri).body(Body::empty()).unwrap()).await
}

pub async fn get_with_cookie(app: &Router, uri: &str, cookie: &str) -> Response<Body> {
    send(
        app,
        Request::get(uri)
            .header(header::COOKIE, cookie)
            .body(Body::empty())
            .unwrap(),
    )
    .await
}

pub async fn post_json(app: &Router, uri: &str, json: &str) -> Response<Body> {
    let request = Request::post(uri)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(json.to_owned()))
        .unwrap();
    send(app, request).await
}

/// Poimii `Set-Cookie`-otsakkeesta `name=value`-osan Cookie-otsaketta varten.
pub fn session_cookie(response: &Response<Body>) -> String {
    response
        .headers()
        .get(header::SET_COOKIE)
        .expect("response should set a cookie")
        .to_str()
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .to_owned()
}

pub async fn body_bytes(response: Response<Body>) -> Bytes {
    response.into_body().collect().await.unwrap().to_bytes()
}

pub async fn body_json(response: Response<Body>) -> serde_json::Value {
    let bytes = body_bytes(response).await;
    serde_json::from_slice(&bytes).expect("response body should be JSON")
}
