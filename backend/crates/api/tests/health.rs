mod common;

use axum::http::StatusCode;
use sqlx::PgPool;

#[sqlx::test(migrator = "api::MIGRATOR")]
async fn health_reports_ok_when_database_is_up(pool: PgPool) {
    let app = common::test_app(pool);

    let response = common::get(&app, "/api/health").await;
    assert_eq!(response.status(), StatusCode::OK);

    let body = common::body_json(response).await;
    assert_eq!(body["status"], "ok");
    assert_eq!(body["database"], "up");
}

#[sqlx::test(migrator = "api::MIGRATOR")]
async fn unknown_route_returns_404(pool: PgPool) {
    let app = common::test_app(pool);

    let response = common::get(&app, "/api/does-not-exist").await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[sqlx::test(migrator = "api::MIGRATOR")]
async fn migrations_create_expected_tables_and_views(pool: PgPool) {
    let tables: Vec<String> = sqlx::query_scalar(
        "SELECT table_name FROM information_schema.tables
         WHERE table_schema = 'public' AND table_type = 'BASE TABLE'
         ORDER BY table_name",
    )
    .fetch_all(&pool)
    .await
    .unwrap();

    for expected in [
        "app_users",
        "polar_accounts",
        "exercises",
        "sleep_nights",
        "nightly_recharge",
        "daily_activity",
        "physical_info",
        "cardio_load",
        "sync_runs",
    ] {
        assert!(
            tables.iter().any(|t| t == expected),
            "missing table {expected}"
        );
    }

    let views: Vec<String> = sqlx::query_scalar(
        "SELECT table_name FROM information_schema.views WHERE table_schema = 'public'",
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    assert!(views.iter().any(|v| v == "v_daily_wellness"));
    assert!(views.iter().any(|v| v == "v_weekly_summary"));
}
