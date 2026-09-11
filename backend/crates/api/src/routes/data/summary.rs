//! Yhteenvedot dashboardia varten: yleiskuva, päivä- ja viikkonäkymät.

use axum::{
    Json,
    extract::{Query, State},
};
use chrono::{DateTime, Duration, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use utoipa_axum::{router::OpenApiRouter, routes};

use super::{MAX_RANGE_DAYS, RangeQuery, primary_account};
use crate::{
    auth::ReadAccess,
    db,
    error::{ApiResult, ErrorBody},
    state::AppState,
};

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(overview))
        .routes(routes!(daily))
        .routes(routes!(weekly))
}

// ---------------------------------------------------------------------------
// Yleiskuva
// ---------------------------------------------------------------------------

#[derive(Debug, Default, Serialize, ToSchema)]
pub struct Latest {
    pub sleep_date: Option<NaiveDate>,
    pub sleep_score: Option<i16>,
    pub sleep_total_s: Option<i32>,
    pub recharge_date: Option<NaiveDate>,
    pub nightly_recharge_status: Option<i16>,
    pub activity_date: Option<NaiveDate>,
    pub steps: Option<i32>,
    pub weight_kg: Option<f32>,
    pub vo2_max: Option<i16>,
    pub resting_heart_rate: Option<i16>,
    pub cardio_load_status: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct Overview {
    /// Onko Polar-tili yhdistetty ja dataa odotettavissa.
    pub polar_connected: bool,
    /// `true`, jos paino ja pituus on piilotettu tästä vastauksesta
    /// (kirjautumaton katselija ja `PUBLIC_BODY_METRICS=false`).
    pub body_metrics_hidden: bool,
    pub last_sync_at: Option<DateTime<Utc>>,
    pub exercises: i64,
    pub sleep_nights: i64,
    pub activity_days: i64,
    /// Vanhin ja uusin päivä, jolta on mitä tahansa dataa.
    pub first_date: Option<NaiveDate>,
    pub last_date: Option<NaiveDate>,
    /// Kannassa esiintyvät lajit suodattimia varten.
    pub sports: Vec<String>,
    pub latest: Latest,
}

/// Dashboardin yleiskuva: määrät, aikaväli, lajit ja tuoreimmat arvot.
#[utoipa::path(get, path = "/summary/overview", tag = "summary",
    responses((status = 200, body = Overview)))]
async fn overview(State(state): State<AppState>, read: ReadAccess) -> ApiResult<Json<Overview>> {
    let hide_body = super::hide_body_metrics(&state, &read);
    let Some(account_rec) = db::polar_accounts::first(&state.pool).await? else {
        return Ok(Json(Overview {
            polar_connected: false,
            body_metrics_hidden: hide_body,
            last_sync_at: None,
            exercises: 0,
            sleep_nights: 0,
            activity_days: 0,
            first_date: None,
            last_date: None,
            sports: vec![],
            latest: Latest::default(),
        }));
    };
    let account = account_rec.id;

    let totals = sqlx::query!(
        r#"SELECT
             (SELECT count(*) FROM exercises      WHERE polar_account_id = $1) AS "exercises!",
             (SELECT count(*) FROM sleep_nights   WHERE polar_account_id = $1) AS "sleep_nights!",
             (SELECT count(*) FROM daily_activity WHERE polar_account_id = $1) AS "activity_days!",
             (SELECT min(date) FROM v_daily_wellness WHERE polar_account_id = $1) AS "first_date?",
             (SELECT max(date) FROM v_daily_wellness WHERE polar_account_id = $1) AS "last_date?""#,
        account
    )
    .fetch_one(&state.pool)
    .await?;

    let sports: Vec<String> = sqlx::query_scalar!(
        "SELECT DISTINCT sport FROM exercises WHERE polar_account_id = $1 ORDER BY sport",
        account
    )
    .fetch_all(&state.pool)
    .await?;

    let sleep = sqlx::query!(
        r#"SELECT date, sleep_score,
                  (light_sleep_s + deep_sleep_s + rem_sleep_s) AS "total_s?"
           FROM sleep_nights WHERE polar_account_id = $1 ORDER BY date DESC LIMIT 1"#,
        account
    )
    .fetch_optional(&state.pool)
    .await?;
    let recharge = sqlx::query!(
        "SELECT date, nightly_recharge_status FROM nightly_recharge
         WHERE polar_account_id = $1 ORDER BY date DESC LIMIT 1",
        account
    )
    .fetch_optional(&state.pool)
    .await?;
    let activity = sqlx::query!(
        "SELECT date, steps FROM daily_activity
         WHERE polar_account_id = $1 ORDER BY date DESC LIMIT 1",
        account
    )
    .fetch_optional(&state.pool)
    .await?;
    let physical = sqlx::query!(
        "SELECT weight_kg, vo2_max, resting_heart_rate FROM physical_info
         WHERE polar_account_id = $1 ORDER BY date DESC LIMIT 1",
        account
    )
    .fetch_optional(&state.pool)
    .await?;
    let cardio = sqlx::query_scalar!(
        r#"SELECT status AS "status?" FROM cardio_load
           WHERE polar_account_id = $1 ORDER BY date DESC LIMIT 1"#,
        account
    )
    .fetch_optional(&state.pool)
    .await?
    .flatten();

    Ok(Json(Overview {
        polar_connected: true,
        body_metrics_hidden: hide_body,
        last_sync_at: account_rec.last_sync_at,
        exercises: totals.exercises,
        sleep_nights: totals.sleep_nights,
        activity_days: totals.activity_days,
        first_date: totals.first_date,
        last_date: totals.last_date,
        sports,
        latest: Latest {
            sleep_date: sleep.as_ref().map(|s| s.date),
            sleep_score: sleep.as_ref().and_then(|s| s.sleep_score),
            sleep_total_s: sleep.as_ref().and_then(|s| s.total_s),
            recharge_date: recharge.as_ref().map(|r| r.date),
            nightly_recharge_status: recharge.as_ref().and_then(|r| r.nightly_recharge_status),
            activity_date: activity.as_ref().map(|a| a.date),
            steps: activity.as_ref().and_then(|a| a.steps),
            weight_kg: if hide_body {
                None
            } else {
                physical.as_ref().and_then(|p| p.weight_kg)
            },
            vo2_max: physical.as_ref().and_then(|p| p.vo2_max),
            resting_heart_rate: physical.as_ref().and_then(|p| p.resting_heart_rate),
            cardio_load_status: cardio,
        },
    }))
}

// ---------------------------------------------------------------------------
// Päivänäkymä (v_daily_wellness)
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, ToSchema)]
pub struct DailyWellness {
    pub date: NaiveDate,
    pub sleep_score: Option<i16>,
    pub sleep_charge: Option<i16>,
    pub sleep_total_s: Option<i32>,
    pub light_sleep_s: Option<i32>,
    pub deep_sleep_s: Option<i32>,
    pub rem_sleep_s: Option<i32>,
    pub total_interruption_s: Option<i32>,
    pub continuity: Option<f32>,
    pub nightly_recharge_status: Option<i16>,
    pub ans_charge: Option<f32>,
    pub ans_charge_status: Option<i16>,
    pub night_hr_avg: Option<i16>,
    pub night_hrv_avg_ms: Option<i16>,
    pub night_breathing_rate_avg: Option<f32>,
    pub steps: Option<i32>,
    pub calories: Option<i32>,
    pub active_calories: Option<i32>,
    pub active_duration_s: Option<i32>,
    pub daily_activity_pct: Option<f32>,
    pub distance_from_steps_m: Option<f32>,
    pub cardio_load_status: Option<String>,
    pub cardio_load: Option<f32>,
    pub strain: Option<f32>,
    pub tolerance: Option<f32>,
    pub cardio_load_ratio: Option<f32>,
}

/// Uni, palautuminen, aktiivisuus ja kuormitus päivittäin yhdellä rivillä.
#[utoipa::path(get, path = "/summary/daily", tag = "summary",
    params(RangeQuery),
    responses((status = 200, body = Vec<DailyWellness>), (status = 400, body = ErrorBody)))]
async fn daily(
    State(state): State<AppState>,
    _read: ReadAccess,
    Query(q): Query<RangeQuery>,
) -> ApiResult<Json<Vec<DailyWellness>>> {
    let (from, to) = q.resolve(30, MAX_RANGE_DAYS)?;
    let Some(account) = primary_account(&state.pool).await? else {
        return Ok(Json(vec![]));
    };
    let rows = sqlx::query_as!(
        DailyWellness,
        r#"SELECT date AS "date!", sleep_score, sleep_charge, sleep_total_s, light_sleep_s,
                  deep_sleep_s, rem_sleep_s, total_interruption_s, continuity,
                  nightly_recharge_status, ans_charge, ans_charge_status, night_hr_avg,
                  night_hrv_avg_ms, night_breathing_rate_avg, steps, calories, active_calories,
                  active_duration_s, daily_activity_pct, distance_from_steps_m,
                  cardio_load_status, cardio_load, strain, tolerance, cardio_load_ratio
           FROM v_daily_wellness
           WHERE polar_account_id = $1 AND date BETWEEN $2 AND $3
           ORDER BY date"#,
        account,
        from,
        to
    )
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(rows))
}

// ---------------------------------------------------------------------------
// Viikkonäkymä (v_weekly_summary)
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, ToSchema)]
pub struct WeeklySummary {
    /// Viikon maanantai.
    pub week_start: NaiveDate,
    pub exercise_count: i64,
    pub total_duration_s: i64,
    pub total_distance_m: Option<f32>,
    pub total_calories: Option<i64>,
    pub total_training_load: Option<f32>,
    pub avg_hr: Option<f64>,
    pub max_hr: Option<i16>,
    pub avg_sleep_score: Option<f64>,
    pub avg_sleep_total_s: Option<f64>,
    pub avg_recharge_status: Option<f64>,
    pub avg_night_hrv_ms: Option<f64>,
    pub total_steps: Option<i64>,
    pub nights_with_sleep: i64,
}

#[derive(Debug, Default, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct WeeksQuery {
    /// Montako viimeistä viikkoa (1–104, oletus 12).
    pub weeks: Option<u32>,
}

/// Viikkoyhteenveto: harjoitusmäärät sekä keskimääräinen uni ja palautuminen.
#[utoipa::path(get, path = "/summary/weekly", tag = "summary",
    params(WeeksQuery),
    responses((status = 200, body = Vec<WeeklySummary>)))]
async fn weekly(
    State(state): State<AppState>,
    _read: ReadAccess,
    Query(q): Query<WeeksQuery>,
) -> ApiResult<Json<Vec<WeeklySummary>>> {
    let weeks = i64::from(q.weeks.unwrap_or(12).clamp(1, 104));
    let Some(account) = primary_account(&state.pool).await? else {
        return Ok(Json(vec![]));
    };
    let since = Utc::now().date_naive() - Duration::weeks(weeks);
    let rows = sqlx::query_as!(
        WeeklySummary,
        r#"SELECT week_start AS "week_start!", exercise_count AS "exercise_count!",
                  total_duration_s AS "total_duration_s!", total_distance_m, total_calories,
                  total_training_load, avg_hr, max_hr, avg_sleep_score, avg_sleep_total_s,
                  avg_recharge_status, avg_night_hrv_ms, total_steps,
                  nights_with_sleep AS "nights_with_sleep!"
           FROM v_weekly_summary
           WHERE polar_account_id = $1 AND week_start >= $2
           ORDER BY week_start"#,
        account,
        since
    )
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(rows))
}
