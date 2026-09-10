mod common;

use axum::{
    body::Body,
    http::{Request, StatusCode, header},
};
use domain::Role;
use sqlx::PgPool;

const EMAIL: &str = "toni@example.com";
const PASSWORD: &str = "correct horse battery staple";

async fn seed_user(pool: &PgPool) {
    let hash = api::auth::password::hash(PASSWORD.into()).await.unwrap();
    api::db::users::insert(pool, EMAIL, &hash, Role::Owner)
        .await
        .unwrap();
}

fn login_body(email: &str, password: &str) -> String {
    serde_json::json!({ "email": email, "password": password }).to_string()
}

#[sqlx::test(migrator = "api::MIGRATOR")]
async fn login_sets_httponly_cookie_and_returns_user(pool: PgPool) {
    seed_user(&pool).await;
    let app = common::test_app(pool);

    let response = common::post_json(&app, "/api/auth/login", &login_body(EMAIL, PASSWORD)).await;
    assert_eq!(response.status(), StatusCode::OK);

    let set_cookie = response
        .headers()
        .get(header::SET_COOKIE)
        .expect("login must set a cookie")
        .to_str()
        .unwrap()
        .to_owned();
    assert!(set_cookie.starts_with("pdh_session="));
    assert!(set_cookie.contains("HttpOnly"));
    assert!(set_cookie.contains("SameSite=Lax"));
    assert!(set_cookie.contains("Path=/"));

    let body = common::body_json(response).await;
    assert_eq!(body["email"], EMAIL);
    assert_eq!(body["role"], "owner");
    assert!(body.get("password_hash").is_none());
}

#[sqlx::test(migrator = "api::MIGRATOR")]
async fn login_is_case_insensitive_for_email(pool: PgPool) {
    seed_user(&pool).await;
    let app = common::test_app(pool);

    let response = common::post_json(
        &app,
        "/api/auth/login",
        &login_body("Toni@Example.COM ", PASSWORD),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
}

#[sqlx::test(migrator = "api::MIGRATOR")]
async fn wrong_password_and_unknown_user_both_return_401(pool: PgPool) {
    seed_user(&pool).await;
    let app = common::test_app(pool);

    let wrong = common::post_json(&app, "/api/auth/login", &login_body(EMAIL, "nope")).await;
    assert_eq!(wrong.status(), StatusCode::UNAUTHORIZED);
    assert!(wrong.headers().get(header::SET_COOKIE).is_none());
    let body = common::body_json(wrong).await;
    assert_eq!(body["error"]["code"], "unauthorized");

    let unknown = common::post_json(
        &app,
        "/api/auth/login",
        &login_body("nobody@example.com", PASSWORD),
    )
    .await;
    assert_eq!(unknown.status(), StatusCode::UNAUTHORIZED);
}

#[sqlx::test(migrator = "api::MIGRATOR")]
async fn empty_credentials_return_400(pool: PgPool) {
    let app = common::test_app(pool);
    let response = common::post_json(&app, "/api/auth/login", &login_body("", "")).await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[sqlx::test(migrator = "api::MIGRATOR")]
async fn me_requires_session_cookie(pool: PgPool) {
    seed_user(&pool).await;
    let app = common::test_app(pool);

    let anonymous = common::get(&app, "/api/auth/me").await;
    assert_eq!(anonymous.status(), StatusCode::UNAUTHORIZED);

    let login = common::post_json(&app, "/api/auth/login", &login_body(EMAIL, PASSWORD)).await;
    let cookie = common::session_cookie(&login);

    let me = common::send(
        &app,
        Request::get("/api/auth/me")
            .header(header::COOKIE, &cookie)
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_eq!(me.status(), StatusCode::OK);
    assert_eq!(common::body_json(me).await["email"], EMAIL);

    let forged = common::send(
        &app,
        Request::get("/api/auth/me")
            .header(header::COOKIE, "pdh_session=not.a.jwt")
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_eq!(forged.status(), StatusCode::UNAUTHORIZED);
}

#[sqlx::test(migrator = "api::MIGRATOR")]
async fn logout_clears_cookie(pool: PgPool) {
    let app = common::test_app(pool);
    let response = common::send(
        &app,
        Request::post("/api/auth/logout")
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let set_cookie = response
        .headers()
        .get(header::SET_COOKIE)
        .unwrap()
        .to_str()
        .unwrap();
    assert!(set_cookie.starts_with("pdh_session="));
    assert!(set_cookie.contains("Max-Age=0"));
}

#[sqlx::test(migrator = "api::MIGRATOR")]
async fn seed_creates_owner_only_when_no_users_exist(pool: PgPool) {
    let config = api::Config {
        admin_email: Some("Owner@Example.com".into()),
        admin_password: Some("initial-password-123".into()),
        ..api::Config::for_tests()
    };
    api::seed::ensure_owner(&pool, &config).await.unwrap();
    assert_eq!(api::db::users::count(&pool).await.unwrap(), 1);

    // Toinen ajo eri sähköpostilla ei luo mitään.
    let again = api::Config {
        admin_email: Some("second@example.com".into()),
        ..config
    };
    api::seed::ensure_owner(&pool, &again).await.unwrap();
    assert_eq!(api::db::users::count(&pool).await.unwrap(), 1);

    let user = api::db::users::find_by_email(&pool, "owner@example.com")
        .await
        .unwrap()
        .expect("owner should exist with lowercased email");
    assert_eq!(user.role, "owner");
}

#[sqlx::test(migrator = "api::MIGRATOR")]
async fn seed_fails_loudly_without_credentials(pool: PgPool) {
    let result = api::seed::ensure_owner(&pool, &api::Config::for_tests()).await;
    assert!(result.is_err());
}
