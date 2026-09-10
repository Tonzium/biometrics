mod common;

use axum::{
    body::Body,
    http::{Request, StatusCode, header},
};
use domain::Role;
use polar_client::PolarConfig;
use sqlx::PgPool;
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{basic_auth, bearer_token, body_string_contains, method, path},
};

const EMAIL: &str = "owner@example.com";
const PASSWORD: &str = "owner-password-123";

async fn seed(pool: &PgPool, role: Role) {
    let hash = api::auth::password::hash(PASSWORD.into()).await.unwrap();
    api::db::users::insert(pool, EMAIL, &hash, role)
        .await
        .unwrap();
}

async fn app_with_mock(pool: PgPool, server: &MockServer) -> axum::Router {
    let config = api::Config {
        polar: Some(PolarConfig::for_base_url(&server.uri())),
        ..api::Config::for_tests()
    };
    common::test_app_with(config, pool)
}

async fn login(app: &axum::Router) -> String {
    let body = serde_json::json!({ "email": EMAIL, "password": PASSWORD }).to_string();
    let response = common::post_json(app, "/api/auth/login", &body).await;
    assert_eq!(response.status(), StatusCode::OK);
    common::session_cookie(&response)
}

fn cookie_value(set_cookie: &str, name: &str) -> Option<String> {
    let first = set_cookie.split(';').next()?;
    let (k, v) = first.split_once('=')?;
    (k == name).then(|| v.to_owned())
}

#[sqlx::test(migrator = "api::MIGRATOR")]
async fn polar_routes_require_login(pool: PgPool) {
    let app = common::test_app(pool);
    for uri in [
        "/api/polar/status",
        "/api/polar/connect",
        "/api/polar/callback",
    ] {
        assert_eq!(
            common::get(&app, uri).await.status(),
            StatusCode::UNAUTHORIZED,
            "{uri}"
        );
    }
    let del = common::send(
        &app,
        Request::delete("/api/polar/disconnect")
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_eq!(del.status(), StatusCode::UNAUTHORIZED);
}

#[sqlx::test(migrator = "api::MIGRATOR")]
async fn connect_is_503_when_polar_is_not_configured(pool: PgPool) {
    seed(&pool, Role::Owner).await;
    let app = common::test_app(pool);
    let session = login(&app).await;

    let status = common::get_with_cookie(&app, "/api/polar/status", &session).await;
    let body = common::body_json(status).await;
    assert_eq!(body["configured"], false);
    assert_eq!(body["connected"], false);

    let connect = common::get_with_cookie(&app, "/api/polar/connect", &session).await;
    assert_eq!(connect.status(), StatusCode::SERVICE_UNAVAILABLE);
}

#[sqlx::test(migrator = "api::MIGRATOR")]
async fn viewer_cannot_connect(pool: PgPool) {
    seed(&pool, Role::Viewer).await;
    let server = MockServer::start().await;
    let app = app_with_mock(pool, &server).await;
    let session = login(&app).await;

    let connect = common::get_with_cookie(&app, "/api/polar/connect", &session).await;
    assert_eq!(connect.status(), StatusCode::FORBIDDEN);
}

#[sqlx::test(migrator = "api::MIGRATOR")]
async fn connect_redirects_to_polar_with_state_cookie(pool: PgPool) {
    seed(&pool, Role::Owner).await;
    let server = MockServer::start().await;
    let app = app_with_mock(pool, &server).await;
    let session = login(&app).await;

    let response = common::get_with_cookie(&app, "/api/polar/connect", &session).await;
    assert_eq!(response.status(), StatusCode::SEE_OTHER);

    let location = response.headers()[header::LOCATION].to_str().unwrap();
    assert!(location.starts_with(&format!("{}/oauth2/authorization?", server.uri())));
    assert!(location.contains("client_id=test-client-id"));
    assert!(location.contains("response_type=code"));

    let set_cookie = response.headers()[header::SET_COOKIE].to_str().unwrap();
    let state = cookie_value(set_cookie, "pdh_oauth_state").expect("state cookie");
    assert!(location.contains(&format!("state={state}")));
    assert!(set_cookie.contains("HttpOnly"));
    assert!(set_cookie.contains("Path=/api/polar"));
}

#[sqlx::test(migrator = "api::MIGRATOR")]
async fn callback_rejects_state_mismatch(pool: PgPool) {
    seed(&pool, Role::Owner).await;
    let server = MockServer::start().await;
    let app = app_with_mock(pool, &server).await;
    let session = login(&app).await;

    let cookie = format!("{session}; pdh_oauth_state=expected");
    let response =
        common::get_with_cookie(&app, "/api/polar/callback?code=abc&state=forged", &cookie).await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[sqlx::test(migrator = "api::MIGRATOR")]
async fn denied_authorization_redirects_to_settings(pool: PgPool) {
    seed(&pool, Role::Owner).await;
    let server = MockServer::start().await;
    let app = app_with_mock(pool, &server).await;
    let session = login(&app).await;

    let response =
        common::get_with_cookie(&app, "/api/polar/callback?error=access_denied", &session).await;
    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    assert_eq!(
        response.headers()[header::LOCATION].to_str().unwrap(),
        "/settings?polar=denied"
    );
}

#[sqlx::test(migrator = "api::MIGRATOR")]
async fn full_connect_and_disconnect_flow(pool: PgPool) {
    seed(&pool, Role::Owner).await;
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/v2/oauth2/token"))
        .and(basic_auth("test-client-id", "test-client-secret"))
        .and(body_string_contains("code=auth-code-1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "access_token": "polar-access-token",
            "token_type": "bearer",
            "x_user_id": 555
        })))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/v3/users"))
        .and(bearer_token("polar-access-token"))
        .respond_with(ResponseTemplate::new(409))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("DELETE"))
        .and(path("/v3/users/555"))
        .and(bearer_token("polar-access-token"))
        .respond_with(ResponseTemplate::new(204))
        .expect(1)
        .mount(&server)
        .await;

    let app = app_with_mock(pool.clone(), &server).await;
    let session = login(&app).await;

    // 1. connect -> state cookie
    let connect = common::get_with_cookie(&app, "/api/polar/connect", &session).await;
    let state = cookie_value(
        connect.headers()[header::SET_COOKIE].to_str().unwrap(),
        "pdh_oauth_state",
    )
    .unwrap();

    // 2. callback with matching state
    let cookie = format!("{session}; pdh_oauth_state={state}");
    let callback = common::get_with_cookie(
        &app,
        &format!("/api/polar/callback?code=auth-code-1&state={state}"),
        &cookie,
    )
    .await;
    assert_eq!(callback.status(), StatusCode::SEE_OTHER);
    assert_eq!(
        callback.headers()[header::LOCATION].to_str().unwrap(),
        "/settings?polar=connected"
    );
    // state cookie is cleared
    let cleared = callback.headers()[header::SET_COOKIE].to_str().unwrap();
    assert!(cleared.starts_with("pdh_oauth_state=;"));
    assert!(cleared.contains("Max-Age=0"));

    // 3. status shows connection; token is stored encrypted and decryptable
    let status =
        common::body_json(common::get_with_cookie(&app, "/api/polar/status", &session).await).await;
    assert_eq!(status["connected"], true);
    assert_eq!(status["polar_user_id"], 555);

    let stored: Vec<u8> = sqlx::query_scalar("SELECT access_token_enc FROM polar_accounts")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_ne!(stored, b"polar-access-token");
    let cipher = api::crypto::TokenCipher::from_key_bytes(&api::Config::for_tests().encryption_key);
    assert_eq!(cipher.decrypt(&stored).unwrap(), "polar-access-token");

    // 4. disconnect
    let disconnect = common::send(
        &app,
        Request::delete("/api/polar/disconnect")
            .header(header::COOKIE, &session)
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_eq!(disconnect.status(), StatusCode::NO_CONTENT);

    let status =
        common::body_json(common::get_with_cookie(&app, "/api/polar/status", &session).await).await;
    assert_eq!(status["connected"], false);

    // 5. disconnecting again is 404
    let again = common::send(
        &app,
        Request::delete("/api/polar/disconnect")
            .header(header::COOKIE, &session)
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_eq!(again.status(), StatusCode::NOT_FOUND);
}

#[sqlx::test(migrator = "api::MIGRATOR")]
async fn callback_redirects_with_error_when_polar_fails(pool: PgPool) {
    seed(&pool, Role::Owner).await;
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v2/oauth2/token"))
        .respond_with(ResponseTemplate::new(400).set_body_string("invalid_grant"))
        .mount(&server)
        .await;

    let app = app_with_mock(pool, &server).await;
    let session = login(&app).await;
    let cookie = format!("{session}; pdh_oauth_state=s1");
    let callback =
        common::get_with_cookie(&app, "/api/polar/callback?code=bad&state=s1", &cookie).await;
    assert_eq!(callback.status(), StatusCode::SEE_OTHER);
    assert_eq!(
        callback.headers()[header::LOCATION].to_str().unwrap(),
        "/settings?polar=error"
    );
}
