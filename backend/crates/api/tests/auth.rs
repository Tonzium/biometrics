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

#[sqlx::test(migrator = "api::MIGRATOR")]
async fn sessions_can_be_invalidated_on_the_server(pool: PgPool) {
    seed_user(&pool).await;
    let app = common::test_app(pool.clone());

    let login = common::post_json(&app, "/api/auth/login", &login_body(EMAIL, PASSWORD)).await;
    let cookie = common::session_cookie(&login);
    let before = common::get_with_cookie(&app, "/api/auth/me", &cookie).await;
    assert_eq!(before.status(), StatusCode::OK);

    // Ylläpitotoimi: kasvata istuntoversiota.
    let user = api::db::users::find_by_email(&pool, EMAIL)
        .await
        .unwrap()
        .expect("seeded user");
    let affected = api::db::users::invalidate_sessions(&pool, user.id)
        .await
        .unwrap();
    assert_eq!(affected, 1);

    // Sama cookie ei enää kelpaa, vaikka token ei ole vanhentunut.
    let after = common::get_with_cookie(&app, "/api/auth/me", &cookie).await;
    assert_eq!(after.status(), StatusCode::UNAUTHORIZED);

    // Uusi kirjautuminen toimii heti.
    let again = common::post_json(&app, "/api/auth/login", &login_body(EMAIL, PASSWORD)).await;
    assert_eq!(again.status(), StatusCode::OK);
    let fresh = common::session_cookie(&again);
    let ok = common::get_with_cookie(&app, "/api/auth/me", &fresh).await;
    assert_eq!(ok.status(), StatusCode::OK);
}

#[sqlx::test(migrator = "api::MIGRATOR")]
async fn deleted_user_cannot_use_an_existing_session(pool: PgPool) {
    seed_user(&pool).await;
    let app = common::test_app(pool.clone());

    let login = common::post_json(&app, "/api/auth/login", &login_body(EMAIL, PASSWORD)).await;
    let cookie = common::session_cookie(&login);

    // Muut testit käyttävät samaa ei-makro-muotoa: makro vaatisi
    // kyselyn myös .sqlx-offline-dataan, jota prepare ei kerää testeistä.
    sqlx::query("DELETE FROM app_users")
        .execute(&pool)
        .await
        .unwrap();

    let after = common::get_with_cookie(&app, "/api/auth/me", &cookie).await;
    assert_eq!(after.status(), StatusCode::UNAUTHORIZED);
}

#[sqlx::test(migrator = "api::MIGRATOR")]
async fn login_is_rejected_with_429_when_concurrency_limit_is_full(pool: PgPool) {
    seed_user(&pool).await;
    let (app, state) = common::test_app_and_state(pool);

    // Onnistunut kirjautuminen ensin: sen jälkeen kaikki permitit ovat taas
    // vapaina, eli onnistumispolku ei vuoda permittiä.
    let ok = common::post_json(&app, "/api/auth/login", &login_body(EMAIL, PASSWORD)).await;
    assert_eq!(ok.status(), StatusCode::OK);
    assert_eq!(
        state.login_limit.available_permits(),
        api::auth::LOGIN_MAX_CONCURRENT,
        "onnistunut kirjautuminen ei saa vuotaa permittiä"
    );

    // Yksi permit vapaana: kirjautuminen toimii edelleen. Tämä sitoo rajan
    // lukuun: jos LOGIN_MAX_CONCURRENT muuttuu, tämä tai 429-tapaus hajoaa.
    let held_all_but_one = state
        .login_limit
        .try_acquire_many(api::auth::LOGIN_MAX_CONCURRENT as u32 - 1)
        .expect("kaikkien permittien pitäisi olla vapaina");
    let last_one = common::post_json(&app, "/api/auth/login", &login_body(EMAIL, PASSWORD)).await;
    assert_eq!(last_one.status(), StatusCode::OK);
    drop(held_all_but_one);

    // Kaikki permitit varattuna testin puolelta: käsittelijä ei saa yhtään.
    let held = state
        .login_limit
        .try_acquire_many(api::auth::LOGIN_MAX_CONCURRENT as u32)
        .expect("kaikkien permittien pitäisi olla vapaina");

    let busy = common::post_json(&app, "/api/auth/login", &login_body(EMAIL, PASSWORD)).await;
    assert_eq!(busy.status(), StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(
        busy.headers()
            .get(header::RETRY_AFTER)
            .expect("429 must carry Retry-After")
            .to_str()
            .unwrap(),
        "1"
    );
    assert!(busy.headers().get(header::SET_COOKIE).is_none());
    let body = common::body_json(busy).await;
    assert_eq!(body["error"]["code"], "too_many_requests");

    // Permitit vapautuvat -> kirjautuminen toimii taas.
    drop(held);
    let again = common::post_json(&app, "/api/auth/login", &login_body(EMAIL, PASSWORD)).await;
    assert_eq!(again.status(), StatusCode::OK);
}
