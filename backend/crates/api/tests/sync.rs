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
    matchers::{bearer_token, method, path},
};

const EMAIL: &str = "owner@example.com";
const PASSWORD: &str = "owner-password-123";
const TOKEN: &str = "polar-access-token";

async fn seed_linked_owner(pool: &PgPool) {
    let hash = api::auth::password::hash(PASSWORD.into()).await.unwrap();
    let user = api::db::users::insert(pool, EMAIL, &hash, Role::Owner)
        .await
        .unwrap();
    let cipher = api::crypto::TokenCipher::from_key_bytes(&api::Config::for_tests().encryption_key);
    let enc = cipher.encrypt(TOKEN).unwrap();
    api::db::polar_accounts::upsert_for_user(pool, user.id, 555, &enc, "member")
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

async fn post_sync(app: &axum::Router, cookie: &str) -> axum::response::Response<Body> {
    common::send(
        app,
        Request::post("/api/sync")
            .header(header::COOKIE, cookie)
            .body(Body::empty())
            .unwrap(),
    )
    .await
}

fn json_ok(body: serde_json::Value) -> ResponseTemplate {
    ResponseTemplate::new(200).set_body_json(body)
}

fn exercises_fixture() -> serde_json::Value {
    serde_json::json!([
        {
            "id": "EX1", "upload_time": "2026-09-08T18:10:00.000Z",
            "polar_user": "https://www.polaraccesslink.com/v3/users/555",
            "device": "Polar Vantage V3", "device_id": "1111AAAA",
            "start_time": "2026-09-08T17:02:11", "start_time_utc_offset": 180,
            "duration": "PT52M13S", "calories": 610, "distance": 10123.4,
            "heart_rate": { "average": 152, "maximum": 181 },
            "training_load": 143.2, "sport": "RUNNING", "has_route": true,
            "detailed_sport_info": "RUNNING", "running-index": 51,
            "fat_percentage": 30, "carbohydrate_percentage": 68, "protein_percentage": 2,
            "heart_rate_zones": [
                { "index": 1, "lower-limit": 100, "upper-limit": 120, "in-zone": "PT5M" },
                { "index": 4, "lower-limit": 160, "upper-limit": 180, "in-zone": "PT30M" }
            ]
        },
        {
            "id": "EX2", "start_time": "2026-09-06T07:30:00", "start_time_utc_offset": 180,
            "duration": "PT1H10M", "calories": 420, "sport": "STRENGTH_TRAINING",
            "heart_rate": { "average": 110, "maximum": 150 }, "has_route": false
        }
    ])
}

fn sleep_fixture() -> serde_json::Value {
    serde_json::json!({ "nights": [
        {
            "polar_user": "https://www.polaraccesslink.com/v3/users/555",
            "date": "2026-09-09", "sleep_start_time": "2026-09-08T23:12:00+03:00",
            "sleep_end_time": "2026-09-09T06:45:00+03:00", "device_id": "1111AAAA",
            "continuity": 3.4, "continuity_class": 3, "light_sleep": 12000, "deep_sleep": 5400,
            "rem_sleep": 6300, "unrecognized_sleep_stage": 300, "sleep_score": 82,
            "total_interruption_duration": 1200, "sleep_charge": 3, "sleep_goal": 28800,
            "sleep_rating": 4, "short_interruption_duration": 900,
            "long_interruption_duration": 300, "sleep_cycles": 5,
            "group_duration_score": 88.0, "group_solidity_score": 79.5,
            "group_regeneration_score": 80.1,
            "hypnogram": { "23:12": 1, "23:40": 2, "00:15": 3 },
            "heart_rate_samples": { "23:15": 58, "23:20": 56 }
        },
        {
            "date": "2026-09-08", "sleep_start_time": "2026-09-07T23:50:00+03:00",
            "sleep_end_time": "2026-09-08T06:20:00+03:00", "sleep_score": 71
        }
    ]})
}

fn recharge_fixture() -> serde_json::Value {
    serde_json::json!({ "recharges": [
        { "date": "2026-09-09", "heart_rate_avg": 52, "beat_to_beat_avg": 1150,
          "heart_rate_variability_avg": 48, "breathing_rate_avg": 13.6,
          "nightly_recharge_status": 4, "ans_charge": 1.2, "ans_charge_status": 4,
          "hrv_samples": { "23:15": 45 }, "breathing_samples": { "23:15": 13.5 } },
        { "date": "2026-09-08", "heart_rate_avg": 55, "nightly_recharge_status": 3 }
    ]})
}

fn activities_fixture() -> serde_json::Value {
    serde_json::json!([
        { "start_time": "2026-09-09T00:00:00", "end_time": "2026-09-09T23:59:59",
          "active_duration": "PT3H11M", "inactive_duration": "PT18H23M30S",
          "daily_activity": 89.1, "calories": 2500, "active_calories": 900, "steps": 8823,
          "inactivity_alert_count": 1, "distance_from_steps": 6590.5 },
        // Pelkkä päivämäärä ilman kellonaikaa: tämä muoto hylättiin
        // tuotannossa, joten se on nyt mukana kiinteänä osana erää.
        { "start_time": "2026-09-08", "steps": 12000, "calories": 2800 }
    ])
}

fn physical_fixture() -> serde_json::Value {
    serde_json::json!({
        "weight": 80.5, "height": 181.0, "created": "2024-06-01T12:00:00Z",
        "modified": "2026-09-01T09:30:00Z", "birthday": "1990-01-01", "gender": "MALE",
        "maximum_heart_rate": 188, "resting_heart_rate": 48, "aerobic_threshold": 140,
        "anaerobic_threshold": 168, "vo2_max": 52, "weight_source": "SOURCE_USER",
        "training_background": "FREQUENT", "typical_day": "MOSTLY_SITTING", "sleep_goal": "PT8H"
    })
}

fn cardio_fixture() -> serde_json::Value {
    serde_json::json!([
        { "date": "2026-09-09", "cardio_load_status": "PRODUCTIVE", "cardio_load": 143.2,
          "strain": 95.1, "tolerance": 88.0, "cardio_load_ratio": 1.08,
          "cardio_load_level": { "very_low": 20.0, "low": 50.0, "medium": 90.0, "high": 120.0, "very-high": 150.0 } },
        { "date": "2026-09-08", "cardio_load_status": "MAINTAINING", "cardio_load": 0.0 }
    ])
}

/// Asentaa kaikki kuusi datareittiä mock-palvelimeen onnistuvina.
async fn mount_all_ok(server: &MockServer) {
    let routes = [
        ("/v3/exercises", exercises_fixture()),
        ("/v3/users/sleep", sleep_fixture()),
        ("/v3/users/nightly-recharge", recharge_fixture()),
        ("/v3/users/activities", activities_fixture()),
        ("/v3/users/physical-info", physical_fixture()),
        ("/v3/users/cardio-load", cardio_fixture()),
    ];
    for (p, body) in routes {
        Mock::given(method("GET"))
            .and(path(p))
            .and(bearer_token(TOKEN))
            .respond_with(json_ok(body))
            .mount(server)
            .await;
    }
}

async fn count(pool: &PgPool, table: &str) -> i64 {
    // Vain testeissä: taulun nimi tulee testikoodista, ei syötteestä.
    sqlx::query_scalar(sqlx::AssertSqlSafe(format!("SELECT count(*) FROM {table}")))
        .fetch_one(pool)
        .await
        .unwrap()
}

#[sqlx::test(migrator = "api::MIGRATOR")]
async fn sync_requires_owner_login_and_linked_account(pool: PgPool) {
    let server = MockServer::start().await;
    let app = app_with_mock(pool.clone(), &server).await;

    let anon = common::send(
        &app,
        Request::post("/api/sync").body(Body::empty()).unwrap(),
    )
    .await;
    assert_eq!(anon.status(), StatusCode::UNAUTHORIZED);

    // Kirjautunut omistaja ilman Polar-tiliä -> 404
    let hash = api::auth::password::hash(PASSWORD.into()).await.unwrap();
    api::db::users::insert(&pool, EMAIL, &hash, Role::Owner)
        .await
        .unwrap();
    let session = login(&app).await;
    assert_eq!(
        post_sync(&app, &session).await.status(),
        StatusCode::NOT_FOUND
    );

    let runs = common::get_with_cookie(&app, "/api/sync/runs", &session).await;
    assert_eq!(runs.status(), StatusCode::OK);
    assert_eq!(common::body_json(runs).await.as_array().unwrap().len(), 0);
}

#[sqlx::test(migrator = "api::MIGRATOR")]
async fn full_sync_stores_all_data_and_is_idempotent(pool: PgPool) {
    seed_linked_owner(&pool).await;
    let server = MockServer::start().await;
    mount_all_ok(&server).await;
    let app = app_with_mock(pool.clone(), &server).await;
    let session = login(&app).await;

    let response = post_sync(&app, &session).await;
    assert_eq!(response.status(), StatusCode::OK);
    let report = common::body_json(response).await;
    assert_eq!(report["status"], "ok", "{report}");
    assert_eq!(report["trigger"], "manual");
    assert_eq!(report["counts"]["exercises"], 2);
    assert_eq!(report["counts"]["sleep_nights"], 2);
    assert_eq!(report["counts"]["nightly_recharge"], 2);
    assert_eq!(report["counts"]["daily_activity"], 2);
    assert_eq!(report["counts"]["physical_info"], 1);
    assert_eq!(report["counts"]["cardio_load"], 2);
    assert_eq!(report["counts"]["skipped"], 0);
    assert_eq!(report["errors"].as_array().unwrap().len(), 0);

    // Purettu sarakemuoto on oikein
    let (duration_s, hr_max, start_time, distance): (
        i32,
        Option<i16>,
        chrono::DateTime<chrono::Utc>,
        Option<f32>,
    ) = sqlx::query_as(
        "SELECT duration_s, hr_max, start_time, distance_m FROM exercises WHERE id = 'EX1'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(duration_s, 52 * 60 + 13);
    assert_eq!(hr_max, Some(181));
    assert_eq!(start_time.to_rfc3339(), "2026-09-08T14:02:11+00:00");
    assert!((distance.unwrap() - 10123.4).abs() < 0.1);

    let (score, light, raw): (Option<i16>, Option<i32>, serde_json::Value) = sqlx::query_as(
        "SELECT sleep_score, light_sleep_s, raw FROM sleep_nights WHERE date = '2026-09-09'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(score, Some(82));
    assert_eq!(light, Some(12000));
    assert_eq!(raw["hypnogram"]["23:40"], 2);

    let (active_s, steps): (Option<i32>, Option<i32>) = sqlx::query_as(
        "SELECT active_duration_s, steps FROM daily_activity WHERE date = '2026-09-09'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(active_s, Some(3 * 3600 + 11 * 60));
    assert_eq!(steps, Some(8823));

    let (weight, sleep_goal): (Option<f32>, Option<i32>) =
        sqlx::query_as("SELECT weight_kg, sleep_goal_s FROM physical_info")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(weight, Some(80.5));
    assert_eq!(sleep_goal, Some(8 * 3600));

    // Näkymät toimivat oikealla datalla
    let wellness_rows = count(&pool, "v_daily_wellness").await;
    assert_eq!(wellness_rows, 2);
    let weekly_rows = count(&pool, "v_weekly_summary").await;
    assert!(weekly_rows >= 1);

    // Toinen ajo ei duplikoi rivejä
    let second = post_sync(&app, &session).await;
    assert_eq!(second.status(), StatusCode::OK);
    assert_eq!(count(&pool, "exercises").await, 2);
    assert_eq!(count(&pool, "sleep_nights").await, 2);
    assert_eq!(count(&pool, "cardio_load").await, 2);
    assert_eq!(count(&pool, "sync_runs").await, 2);

    // Ajoloki ja last_sync_at
    let runs =
        common::body_json(common::get_with_cookie(&app, "/api/sync/runs", &session).await).await;
    let runs = runs.as_array().unwrap();
    assert_eq!(runs.len(), 2);
    assert_eq!(runs[0]["status"], "ok");
    assert_eq!(runs[0]["counts"]["exercises"], 2);
    assert!(runs[0]["finished_at"].is_string());

    let status =
        common::body_json(common::get_with_cookie(&app, "/api/polar/status", &session).await).await;
    assert!(status["last_sync_at"].is_string());
}

#[sqlx::test(migrator = "api::MIGRATOR")]
async fn one_failing_endpoint_gives_partial_status(pool: PgPool) {
    seed_linked_owner(&pool).await;
    let server = MockServer::start().await;
    mount_all_ok(&server).await;
    // Uudempi mock voittaa samalle polulle: cardio-load kaatuu
    Mock::given(method("GET"))
        .and(path("/v3/users/cardio-load"))
        .respond_with(ResponseTemplate::new(500).set_body_string("boom"))
        .with_priority(1)
        .mount(&server)
        .await;

    let app = app_with_mock(pool.clone(), &server).await;
    let session = login(&app).await;
    let report = common::body_json(post_sync(&app, &session).await).await;
    assert_eq!(report["status"], "partial", "{report}");
    assert_eq!(report["counts"]["exercises"], 2);
    assert_eq!(report["counts"]["cardio_load"], 0);
    let errors = report["errors"].as_array().unwrap();
    assert_eq!(errors.len(), 1);
    assert!(errors[0].as_str().unwrap().starts_with("cardio_load:"));

    let (status, error): (String, Option<String>) =
        sqlx::query_as("SELECT status, error FROM sync_runs")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(status, "partial");
    assert!(error.unwrap().contains("500"));
}

#[sqlx::test(migrator = "api::MIGRATOR")]
async fn rate_limit_aborts_the_run_as_failed(pool: PgPool) {
    seed_linked_owner(&pool).await;
    let server = MockServer::start().await;
    mount_all_ok(&server).await;
    Mock::given(method("GET"))
        .and(path("/v3/exercises"))
        .respond_with(ResponseTemplate::new(429).insert_header("RateLimit-Reset", "600"))
        .with_priority(1)
        .expect(1)
        .mount(&server)
        .await;
    // Loppuja ei saa edes kutsua
    Mock::given(method("GET"))
        .and(path("/v3/users/sleep"))
        .respond_with(json_ok(sleep_fixture()))
        .with_priority(1)
        .expect(0)
        .mount(&server)
        .await;

    let app = app_with_mock(pool.clone(), &server).await;
    let session = login(&app).await;
    let report = common::body_json(post_sync(&app, &session).await).await;
    assert_eq!(report["status"], "failed", "{report}");
    assert_eq!(count(&pool, "sleep_nights").await, 0);

    let last_sync: Option<chrono::DateTime<chrono::Utc>> =
        sqlx::query_scalar("SELECT last_sync_at FROM polar_accounts")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!(
        last_sync.is_none(),
        "failed run must not update last_sync_at"
    );
}

#[sqlx::test(migrator = "api::MIGRATOR")]
async fn unparseable_items_are_skipped_not_fatal(pool: PgPool) {
    seed_linked_owner(&pool).await;
    let server = MockServer::start().await;
    mount_all_ok(&server).await;
    Mock::given(method("GET"))
        .and(path("/v3/exercises"))
        .respond_with(json_ok(serde_json::json!([
            { "id": "GOOD", "start_time": "2026-09-01T10:00:00", "duration": "PT1H", "sport": "OTHER" },
            { "id": "BAD_DURATION", "start_time": "2026-09-01T10:00:00", "duration": "nonsense", "sport": "OTHER" },
            { "id": "MISSING_FIELDS" }
        ])))
        .with_priority(1)
        .mount(&server)
        .await;

    let app = app_with_mock(pool.clone(), &server).await;
    let session = login(&app).await;
    let report = common::body_json(post_sync(&app, &session).await).await;
    assert_eq!(report["status"], "ok");
    assert_eq!(report["counts"]["exercises"], 1);
    assert_eq!(report["counts"]["skipped"], 2);
}

/// Jos koko erä jää jäsentymättä, ajo ei ole `ok`: tuotannossa kaikki 28
/// aktiivisuuspäivää hylättiin ja raportti näytti silti onnistunutta.
#[sqlx::test(migrator = "api::MIGRATOR")]
async fn a_step_that_stores_nothing_is_partial_not_ok(pool: PgPool) {
    seed_linked_owner(&pool).await;
    let server = MockServer::start().await;
    mount_all_ok(&server).await;
    Mock::given(method("GET"))
        .and(path("/v3/exercises"))
        .respond_with(json_ok(serde_json::json!([
            { "id": "BAD_1" },
            { "id": "BAD_2" }
        ])))
        .with_priority(1)
        .mount(&server)
        .await;

    let app = app_with_mock(pool.clone(), &server).await;
    let session = login(&app).await;
    let report = common::body_json(post_sync(&app, &session).await).await;

    assert_eq!(report["status"], "partial", "{report}");
    assert_eq!(report["counts"]["exercises"], 0);
    assert_eq!(report["counts"]["skipped"], 2);
    let errors = report["errors"].as_array().unwrap();
    assert!(
        errors
            .iter()
            .any(|e| e.as_str().unwrap() == "exercises: fetched 2 items, stored none"),
        "{report}"
    );

    // Muut datatyypit tallentuivat normaalisti, eli yksi hajonnut askel ei
    // keskeytä ajoa.
    assert_eq!(report["counts"]["sleep_nights"], 2);
}

#[sqlx::test(migrator = "api::MIGRATOR")]
async fn sync_all_runs_for_linked_accounts(pool: PgPool) {
    seed_linked_owner(&pool).await;
    let server = MockServer::start().await;
    mount_all_ok(&server).await;
    let config = api::Config {
        polar: Some(PolarConfig::for_base_url(&server.uri())),
        ..api::Config::for_tests()
    };
    let state = api::AppState::new(config, pool.clone()).unwrap();

    api::sync::sync_all(&state, api::sync::Trigger::Scheduled).await;

    let (trigger, status): (String, String) =
        sqlx::query_as("SELECT trigger, status FROM sync_runs")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(trigger, "scheduled");
    assert_eq!(status, "ok");
    assert_eq!(count(&pool, "exercises").await, 2);
}
