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
    let config = Config {
        database_url: String::new(), // ei käytetä: pool annetaan suoraan
        bind_addr: "127.0.0.1:0".into(),
    };
    api::app(AppState::new(config, pool))
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

pub async fn post_json(app: &Router, uri: &str, json: &str) -> Response<Body> {
    let request = Request::post(uri)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(json.to_owned()))
        .unwrap();
    send(app, request).await
}

pub async fn body_bytes(response: Response<Body>) -> Bytes {
    response.into_body().collect().await.unwrap().to_bytes()
}

pub async fn body_json(response: Response<Body>) -> serde_json::Value {
    let bytes = body_bytes(response).await;
    serde_json::from_slice(&bytes).expect("response body should be JSON")
}
