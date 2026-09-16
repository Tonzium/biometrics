mod common;

use axum::http::StatusCode;
use chrono::{Duration, Utc};
use domain::Role;
use polar_client::models::{
    CardioLoad, DailyActivity, Exercise, Fetched, NightlyRecharge, PhysicalInfo, SleepNight,
};
use sqlx::PgPool;
use uuid::Uuid;

const EMAIL: &str = "owner@example.com";
const PASSWORD: &str = "owner-password-123";

fn fetched<T: serde::de::DeserializeOwned>(raw: serde_json::Value) -> Fetched<T> {
    Fetched {
        data: serde_json::from_value(raw.clone()).unwrap(),
        raw,
    }
}

fn day(offset: i64) -> String {
    (Utc::now().date_naive() - Duration::days(offset)).to_string()
}

/// Luo omistajan, Polar-tilin ja pienen mutta kattavan datasetin viimeisiltä päiviltä.
async fn seed(pool: &PgPool) -> Uuid {
    let hash = api::auth::password::hash(PASSWORD.into()).await.unwrap();
    let user = api::db::users::insert(pool, EMAIL, &hash, Role::Owner)
        .await
        .unwrap();
    let account = api::db::polar_accounts::upsert_for_user(pool, user.id, 555, b"enc", "m")
        .await
        .unwrap();
    let id = account.id;

    for (i, (sport, dur)) in [
        ("RUNNING", "PT50M"),
        ("CYCLING", "PT2H"),
        ("RUNNING", "PT30M"),
    ]
    .iter()
    .enumerate()
    {
        let ex: Fetched<Exercise> = fetched(serde_json::json!({
            "id": format!("EX{i}"), "start_time": format!("{}T08:00:00", day(i as i64 * 3)),
            "start_time_utc_offset": 180, "duration": dur, "sport": sport,
            "distance": 5000.0 * (i as f64 + 1.0), "calories": 300 + i,
            "heart_rate": { "average": 140 + i, "maximum": 170 + i },
            "device_id": "SECRET-DEVICE"
        }));
        api::db::polar_data::upsert_exercise(pool, id, &ex)
            .await
            .unwrap();
    }
    for i in 0..5 {
        let n: Fetched<SleepNight> = fetched(serde_json::json!({
            "date": day(i), "sleep_start_time": format!("{}T23:00:00+03:00", day(i + 1)),
            "sleep_end_time": format!("{}T06:30:00+03:00", day(i)),
            "light_sleep": 12000, "deep_sleep": 6000, "rem_sleep": 5000, "sleep_score": 70 + i,
            "hypnogram": { "23:00": 1 }
        }));
        api::db::polar_data::upsert_sleep(pool, id, &n)
            .await
            .unwrap();
        let r: Fetched<NightlyRecharge> = fetched(serde_json::json!({
            "date": day(i), "heart_rate_avg": 50 + i, "heart_rate_variability_avg": 40,
            "nightly_recharge_status": 3
        }));
        api::db::polar_data::upsert_recharge(pool, id, &r)
            .await
            .unwrap();
        let a: Fetched<DailyActivity> = fetched(serde_json::json!({
            "start_time": format!("{}T00:00:00", day(i)), "end_time": format!("{}T23:59:59", day(i)),
            "steps": 8000 + i * 100, "calories": 2400, "active_duration": "PT2H"
        }));
        api::db::polar_data::upsert_activity(pool, id, &a)
            .await
            .unwrap();
        let c: Fetched<CardioLoad> = fetched(serde_json::json!({
            "date": day(i), "cardio_load_status": "PRODUCTIVE", "strain": 90.0, "tolerance": 85.0
        }));
        api::db::polar_data::upsert_cardio_load(pool, id, &c)
            .await
            .unwrap();
    }
    let p: Fetched<PhysicalInfo> = fetched(serde_json::json!({
        "modified": format!("{}T09:00:00Z", day(2)), "weight": 79.5, "vo2_max": 51,
        "resting_heart_rate": 47
    }));
    api::db::polar_data::upsert_physical_info(pool, id, &p)
        .await
        .unwrap();
    id
}

async fn login(app: &axum::Router) -> String {
    let body = serde_json::json!({ "email": EMAIL, "password": PASSWORD }).to_string();
    let response = common::post_json(app, "/api/auth/login", &body).await;
    assert_eq!(response.status(), StatusCode::OK);
    common::session_cookie(&response)
}

#[sqlx::test(migrator = "api::MIGRATOR")]
async fn public_routes_work_without_login(pool: PgPool) {
    seed(&pool).await;
    let app = common::test_app(pool);

    for uri in [
        "/api/exercises",
        "/api/exercises/EX0",
        "/api/sleep",
        "/api/recharge",
        "/api/activity",
        "/api/cardio-load",
        "/api/physical",
        "/api/summary/overview",
        "/api/summary/daily",
        "/api/summary/weekly",
        "/api/meta",
    ] {
        let response = common::get(&app, uri).await;
        assert_eq!(response.status(), StatusCode::OK, "{uri}");
    }
}

#[sqlx::test(migrator = "api::MIGRATOR")]
async fn public_read_false_requires_login(pool: PgPool) {
    seed(&pool).await;
    let config = api::Config {
        public_read: false,
        ..api::Config::for_tests()
    };
    let app = common::test_app_with(config, pool);

    assert_eq!(
        common::get(&app, "/api/exercises").await.status(),
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        common::get(&app, "/api/summary/overview").await.status(),
        StatusCode::UNAUTHORIZED
    );
    // meta on aina julkinen, jotta frontend tietää näyttää kirjautumisen
    let meta = common::body_json(common::get(&app, "/api/meta").await).await;
    assert_eq!(meta["public_read"], false);

    let session = login(&app).await;
    let ok = common::get_with_cookie(&app, "/api/exercises", &session).await;
    assert_eq!(ok.status(), StatusCode::OK);
}

#[sqlx::test(migrator = "api::MIGRATOR")]
async fn exercises_paginate_and_filter(pool: PgPool) {
    seed(&pool).await;
    let app = common::test_app(pool);

    let all = common::body_json(common::get(&app, "/api/exercises").await).await;
    assert_eq!(all["total"], 3);
    assert_eq!(all["items"].as_array().unwrap().len(), 3);
    assert_eq!(all["items"][0]["id"], "EX0", "newest first");
    assert!(
        all["items"][0].get("device_id").is_none(),
        "device id must not leak"
    );
    assert!(all["items"][0].get("raw").is_none());
    assert_eq!(all["items"][0]["duration_s"], 3000);

    let page2 =
        common::body_json(common::get(&app, "/api/exercises?page=2&per_page=2").await).await;
    assert_eq!(page2["page"], 2);
    assert_eq!(page2["items"].as_array().unwrap().len(), 1);
    assert_eq!(page2["total"], 3);

    let running = common::body_json(common::get(&app, "/api/exercises?sport=running").await).await;
    assert_eq!(running["total"], 2);

    let recent =
        common::body_json(common::get(&app, &format!("/api/exercises?from={}", day(1))).await)
            .await;
    assert_eq!(recent["total"], 1);

    let bad = common::get(
        &app,
        &format!("/api/exercises?from={}&to={}", day(0), day(5)),
    )
    .await;
    assert_eq!(bad.status(), StatusCode::BAD_REQUEST);
}

#[sqlx::test(migrator = "api::MIGRATOR")]
async fn exercise_detail_and_404(pool: PgPool) {
    seed(&pool).await;
    let app = common::test_app(pool);

    let ex = common::body_json(common::get(&app, "/api/exercises/EX1").await).await;
    assert_eq!(ex["sport"], "CYCLING");
    assert_eq!(ex["hr_max"], 171);
    assert_eq!(
        common::get(&app, "/api/exercises/NOPE").await.status(),
        StatusCode::NOT_FOUND
    );
}

#[sqlx::test(migrator = "api::MIGRATOR")]
async fn ranged_lists_default_to_30_days_and_validate(pool: PgPool) {
    seed(&pool).await;
    let app = common::test_app(pool);

    let sleep = common::body_json(common::get(&app, "/api/sleep").await).await;
    let nights = sleep.as_array().unwrap();
    assert_eq!(nights.len(), 5);
    assert_eq!(nights[0]["date"], day(4), "ascending by date");
    assert!(nights[0]["hypnogram"].is_object());

    let two =
        common::body_json(common::get(&app, &format!("/api/sleep?from={}", day(1))).await).await;
    assert_eq!(two.as_array().unwrap().len(), 2);

    let too_long = common::get(&app, "/api/sleep?from=2020-01-01&to=2026-01-01").await;
    assert_eq!(too_long.status(), StatusCode::BAD_REQUEST);
    let reversed = common::get(
        &app,
        &format!("/api/recharge?from={}&to={}", day(0), day(3)),
    )
    .await;
    assert_eq!(reversed.status(), StatusCode::BAD_REQUEST);
    let garbage = common::get(&app, "/api/activity?from=yesterday").await;
    assert_eq!(garbage.status(), StatusCode::BAD_REQUEST);

    let activity = common::body_json(common::get(&app, "/api/activity").await).await;
    assert_eq!(activity[4]["steps"], 8000);
    assert_eq!(activity[4]["active_duration_s"], 7200);

    let cardio = common::body_json(common::get(&app, "/api/cardio-load").await).await;
    assert_eq!(cardio[0]["status"], "PRODUCTIVE");

    let physical = common::body_json(common::get(&app, "/api/physical").await).await;
    assert!(
        physical[0]["weight_kg"].is_null(),
        "anonymous must not see weight"
    );
    assert_eq!(physical[0]["resting_heart_rate"], 47);
}

/// Regressio: `to` NaiveDaten alarajalla (`-262143-01-01`) panikoi käsittelijän,
/// koska 30 päivän oletusalku laskettiin siitä vähentämällä ilman tarkistusta.
/// Panic katkaisi yhteyden (nginx vastasi 502) ja jätti pinon api:n lokiin,
/// eikä pyyntö vaatinut kirjautumista. Nyt vastaus on tavallinen 400.
#[sqlx::test(migrator = "api::MIGRATOR")]
async fn out_of_range_to_is_a_bad_request_not_a_panic(pool: PgPool) {
    seed(&pool).await;
    let app = common::test_app(pool);

    for uri in [
        "/api/sleep?to=-262143-01-01",
        "/api/recharge?to=-262143-01-01",
        "/api/activity?to=-262143-01-01",
        "/api/cardio-load?to=-262143-01-01",
        "/api/summary/daily?to=-262143-01-01",
    ] {
        let response = common::get(&app, uri).await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST, "{uri}");
    }
}

#[sqlx::test(migrator = "api::MIGRATOR")]
async fn summaries_reflect_seeded_data(pool: PgPool) {
    seed(&pool).await;
    let app = common::test_app(pool);

    let overview = common::body_json(common::get(&app, "/api/summary/overview").await).await;
    assert_eq!(overview["polar_connected"], true);
    assert_eq!(overview["exercises"], 3);
    assert_eq!(overview["sleep_nights"], 5);
    assert_eq!(overview["activity_days"], 5);
    assert_eq!(
        overview["sports"],
        serde_json::json!(["CYCLING", "RUNNING"])
    );
    assert_eq!(overview["last_date"], day(0));
    assert_eq!(overview["first_date"], day(4));
    assert_eq!(overview["latest"]["sleep_score"], 70);
    assert_eq!(overview["latest"]["sleep_total_s"], 23000);
    assert_eq!(overview["latest"]["steps"], 8000);
    assert!(
        overview["latest"]["weight_kg"].is_null(),
        "anonymous must not see weight"
    );
    assert_eq!(overview["body_metrics_hidden"], true);
    assert_eq!(overview["latest"]["cardio_load_status"], "PRODUCTIVE");

    let daily = common::body_json(common::get(&app, "/api/summary/daily").await).await;
    let days = daily.as_array().unwrap();
    assert_eq!(days.len(), 5);
    let today = days.last().unwrap();
    assert_eq!(today["date"], day(0));
    assert_eq!(today["sleep_total_s"], 23000);
    assert_eq!(today["steps"], 8000);
    assert_eq!(today["night_hr_avg"], 50);
    assert_eq!(today["strain"], 90.0);

    let weekly = common::body_json(common::get(&app, "/api/summary/weekly?weeks=4").await).await;
    let weeks = weekly.as_array().unwrap();
    assert!(!weeks.is_empty());
    let total_ex: i64 = weeks
        .iter()
        .map(|w| w["exercise_count"].as_i64().unwrap())
        .sum();
    assert_eq!(total_ex, 3);
    let total_dur: i64 = weeks
        .iter()
        .map(|w| w["total_duration_s"].as_i64().unwrap())
        .sum();
    assert_eq!(total_dur, 3000 + 7200 + 1800);
}

#[sqlx::test(migrator = "api::MIGRATOR")]
async fn body_metrics_hidden_from_anonymous_by_default(pool: PgPool) {
    seed(&pool).await;
    let app = common::test_app(pool);

    // Kirjautumaton: paino piilotettu, muut arvot näkyvät
    let overview = common::body_json(common::get(&app, "/api/summary/overview").await).await;
    assert_eq!(overview["body_metrics_hidden"], true);
    assert!(overview["latest"]["weight_kg"].is_null());
    assert_eq!(overview["latest"]["vo2_max"], 51);
    let physical = common::body_json(common::get(&app, "/api/physical").await).await;
    assert!(physical[0]["weight_kg"].is_null());
    assert!(physical[0]["height_cm"].is_null());
    assert_eq!(physical[0]["resting_heart_rate"], 47);

    // Kirjautunut näkee painon
    let session = login(&app).await;
    let overview =
        common::body_json(common::get_with_cookie(&app, "/api/summary/overview", &session).await)
            .await;
    assert_eq!(overview["body_metrics_hidden"], false);
    assert_eq!(overview["latest"]["weight_kg"], 79.5);
    let physical =
        common::body_json(common::get_with_cookie(&app, "/api/physical", &session).await).await;
    assert_eq!(physical[0]["weight_kg"], 79.5);
}

#[sqlx::test(migrator = "api::MIGRATOR")]
async fn body_metrics_public_when_enabled(pool: PgPool) {
    seed(&pool).await;
    let config = api::Config {
        public_body_metrics: true,
        ..api::Config::for_tests()
    };
    let app = common::test_app_with(config, pool);
    let overview = common::body_json(common::get(&app, "/api/summary/overview").await).await;
    assert_eq!(overview["body_metrics_hidden"], false);
    assert_eq!(overview["latest"]["weight_kg"], 79.5);
    let meta = common::body_json(common::get(&app, "/api/meta").await).await;
    assert_eq!(meta["public_body_metrics"], true);
}

#[sqlx::test(migrator = "api::MIGRATOR")]
async fn empty_database_returns_empty_not_errors(pool: PgPool) {
    let app = common::test_app(pool);
    let overview = common::body_json(common::get(&app, "/api/summary/overview").await).await;
    assert_eq!(overview["polar_connected"], false);
    assert_eq!(overview["exercises"], 0);
    let ex = common::body_json(common::get(&app, "/api/exercises").await).await;
    assert_eq!(ex["total"], 0);
    assert_eq!(
        common::get(&app, "/api/exercises/X").await.status(),
        StatusCode::NOT_FOUND
    );
    let sleep = common::body_json(common::get(&app, "/api/sleep").await).await;
    assert_eq!(sleep.as_array().unwrap().len(), 0);
}

#[sqlx::test(migrator = "api::MIGRATOR")]
async fn openapi_spec_and_swagger_ui_are_served(pool: PgPool) {
    let app = common::test_app(pool);

    let spec = common::get(&app, "/api/openapi.json").await;
    assert_eq!(spec.status(), StatusCode::OK);
    let spec = common::body_json(spec).await;
    assert!(spec["openapi"].as_str().unwrap().starts_with("3."));
    let paths = spec["paths"].as_object().unwrap();
    for p in [
        "/api/health",
        "/api/meta",
        "/api/auth/login",
        "/api/polar/connect",
        "/api/sync",
        "/api/exercises",
        "/api/exercises/{id}",
        "/api/sleep",
        "/api/summary/overview",
        "/api/summary/weekly",
    ] {
        assert!(paths.contains_key(p), "missing {p} in spec");
    }
    assert!(spec["components"]["schemas"]["Exercise"].is_object());
    assert!(spec["components"]["schemas"]["ErrorBody"].is_object());

    let docs = common::get(&app, "/api/docs/").await;
    assert_eq!(docs.status(), StatusCode::OK);
}
