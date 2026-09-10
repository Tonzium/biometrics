//! Yksittäisten datatyyppien listat: harjoitukset, uni, palautuminen,
//! aktiivisuus, cardio load ja fyysiset tiedot.

use axum::{
    Json,
    extract::{Path, Query, State},
};
use chrono::{DateTime, NaiveDate, NaiveDateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::{IntoParams, ToSchema};
use utoipa_axum::{router::OpenApiRouter, routes};

use super::{MAX_RANGE_DAYS, Paged, RangeQuery, primary_account};
use crate::{
    auth::ReadAccess,
    error::{ApiError, ApiResult, ErrorBody},
    state::AppState,
};

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(list_exercises))
        .routes(routes!(get_exercise))
        .routes(routes!(list_sleep))
        .routes(routes!(list_recharge))
        .routes(routes!(list_activity))
        .routes(routes!(list_cardio_load))
        .routes(routes!(list_physical))
}

// ---------------------------------------------------------------------------
// Harjoitukset
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, ToSchema)]
pub struct Exercise {
    pub id: String,
    /// Alkuhetki UTC.
    pub start_time: DateTime<Utc>,
    /// Alkuhetki paikallisena aikana (kellon aikavyöhyke).
    pub start_time_local: NaiveDateTime,
    pub utc_offset_min: i32,
    pub upload_time: Option<DateTime<Utc>>,
    pub duration_s: i32,
    pub sport: String,
    pub detailed_sport_info: Option<String>,
    pub device: Option<String>,
    pub distance_m: Option<f32>,
    pub calories: Option<i32>,
    pub hr_avg: Option<i16>,
    pub hr_max: Option<i16>,
    pub training_load: Option<f32>,
    pub has_route: bool,
    pub running_index: Option<i16>,
    pub fat_percentage: Option<i16>,
    pub carbohydrate_percentage: Option<i16>,
    pub protein_percentage: Option<i16>,
    /// Polarin sykevyöhykkeet sellaisenaan (`index`, `lower-limit`, `upper-limit`, `in-zone`).
    #[schema(value_type = Option<Object>)]
    pub heart_rate_zones: Option<Value>,
    #[schema(value_type = Option<Object>)]
    pub training_load_pro: Option<Value>,
}

#[derive(Debug, Default, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct ExerciseQuery {
    pub from: Option<NaiveDate>,
    pub to: Option<NaiveDate>,
    /// Lajisuodatin, esim. `RUNNING`.
    pub sport: Option<String>,
    /// Sivunumero alkaen 1:stä.
    pub page: Option<u32>,
    /// Rivejä per sivu (1–200, oletus 50).
    pub per_page: Option<u32>,
}

/// Harjoitukset uusin ensin, sivutettuna.
#[utoipa::path(
    get, path = "/exercises", tag = "data",
    params(ExerciseQuery),
    responses((status = 200, body = Paged<Exercise>), (status = 400, body = ErrorBody))
)]
async fn list_exercises(
    State(state): State<AppState>,
    _read: ReadAccess,
    Query(q): Query<ExerciseQuery>,
) -> ApiResult<Json<Paged<Exercise>>> {
    let page = q.page.unwrap_or(1).max(1);
    let per_page = q.per_page.unwrap_or(50).clamp(1, 200);
    if let (Some(from), Some(to)) = (q.from, q.to)
        && from > to
    {
        return Err(ApiError::BadRequest("`from` must not be after `to`".into()));
    }
    let sport = q.sport.as_deref().map(str::to_uppercase);

    let Some(account) = primary_account(&state.pool).await? else {
        return Ok(Json(Paged {
            items: vec![],
            page,
            per_page,
            total: 0,
        }));
    };

    let total = sqlx::query_scalar!(
        r#"SELECT count(*) AS "count!" FROM exercises
           WHERE polar_account_id = $1
             AND ($2::date IS NULL OR start_time >= $2::date)
             AND ($3::date IS NULL OR start_time < ($3::date + 1))
             AND ($4::text IS NULL OR sport = $4)"#,
        account,
        q.from,
        q.to,
        sport
    )
    .fetch_one(&state.pool)
    .await?;

    let items = sqlx::query_as!(
        Exercise,
        r#"SELECT id, start_time, start_time_local, utc_offset_min, upload_time, duration_s, sport,
                  detailed_sport_info, device, distance_m, calories, hr_avg, hr_max, training_load,
                  has_route, running_index, fat_percentage, carbohydrate_percentage,
                  protein_percentage, heart_rate_zones, training_load_pro
           FROM exercises
           WHERE polar_account_id = $1
             AND ($2::date IS NULL OR start_time >= $2::date)
             AND ($3::date IS NULL OR start_time < ($3::date + 1))
             AND ($4::text IS NULL OR sport = $4)
           ORDER BY start_time DESC
           LIMIT $5 OFFSET $6"#,
        account,
        q.from,
        q.to,
        sport,
        i64::from(per_page),
        i64::from((page - 1) * per_page)
    )
    .fetch_all(&state.pool)
    .await?;

    Ok(Json(Paged {
        items,
        page,
        per_page,
        total,
    }))
}

/// Yksittäinen harjoitus.
#[utoipa::path(
    get, path = "/exercises/{id}", tag = "data",
    params(("id" = String, Path, description = "Polarin harjoitus-id")),
    responses((status = 200, body = Exercise), (status = 404, body = ErrorBody))
)]
async fn get_exercise(
    State(state): State<AppState>,
    _read: ReadAccess,
    Path(id): Path<String>,
) -> ApiResult<Json<Exercise>> {
    let Some(account) = primary_account(&state.pool).await? else {
        return Err(ApiError::NotFound("exercise not found".into()));
    };
    let exercise = sqlx::query_as!(
        Exercise,
        r#"SELECT id, start_time, start_time_local, utc_offset_min, upload_time, duration_s, sport,
                  detailed_sport_info, device, distance_m, calories, hr_avg, hr_max, training_load,
                  has_route, running_index, fat_percentage, carbohydrate_percentage,
                  protein_percentage, heart_rate_zones, training_load_pro
           FROM exercises WHERE polar_account_id = $1 AND id = $2"#,
        account,
        id
    )
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| ApiError::NotFound("exercise not found".into()))?;
    Ok(Json(exercise))
}

// ---------------------------------------------------------------------------
// Uni
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, ToSchema)]
pub struct SleepNight {
    pub date: NaiveDate,
    pub sleep_start_time: DateTime<Utc>,
    pub sleep_end_time: DateTime<Utc>,
    pub continuity: Option<f32>,
    pub continuity_class: Option<i16>,
    pub light_sleep_s: Option<i32>,
    pub deep_sleep_s: Option<i32>,
    pub rem_sleep_s: Option<i32>,
    pub unrecognized_sleep_s: Option<i32>,
    pub sleep_score: Option<i16>,
    pub sleep_charge: Option<i16>,
    pub sleep_goal_s: Option<i32>,
    pub sleep_rating: Option<i16>,
    pub total_interruption_s: Option<i32>,
    pub short_interruption_s: Option<i32>,
    pub long_interruption_s: Option<i32>,
    pub sleep_cycles: Option<i16>,
    pub group_duration_score: Option<f32>,
    pub group_solidity_score: Option<f32>,
    pub group_regeneration_score: Option<f32>,
    /// Univaiheet aikaleimoittain, esim. `{"23:12": 1, "23:40": 2}`.
    #[schema(value_type = Option<Object>)]
    pub hypnogram: Option<Value>,
    /// Syke 5 min välein yön aikana.
    #[schema(value_type = Option<Object>)]
    pub heart_rate_samples: Option<Value>,
}

/// Yöt aikaväliltä (oletus viimeiset 30 päivää).
#[utoipa::path(
    get, path = "/sleep", tag = "data",
    params(RangeQuery),
    responses((status = 200, body = Vec<SleepNight>), (status = 400, body = ErrorBody))
)]
async fn list_sleep(
    State(state): State<AppState>,
    _read: ReadAccess,
    Query(q): Query<RangeQuery>,
) -> ApiResult<Json<Vec<SleepNight>>> {
    let (from, to) = q.resolve(30, MAX_RANGE_DAYS)?;
    let Some(account) = primary_account(&state.pool).await? else {
        return Ok(Json(vec![]));
    };
    let rows = sqlx::query_as!(
        SleepNight,
        r#"SELECT date, sleep_start_time, sleep_end_time, continuity, continuity_class,
                  light_sleep_s, deep_sleep_s, rem_sleep_s, unrecognized_sleep_s, sleep_score,
                  sleep_charge, sleep_goal_s, sleep_rating, total_interruption_s,
                  short_interruption_s, long_interruption_s, sleep_cycles, group_duration_score,
                  group_solidity_score, group_regeneration_score, hypnogram, heart_rate_samples
           FROM sleep_nights
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
// Nightly Recharge
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, ToSchema)]
pub struct NightlyRecharge {
    pub date: NaiveDate,
    pub heart_rate_avg: Option<i16>,
    pub beat_to_beat_avg_ms: Option<i16>,
    pub hrv_avg_ms: Option<i16>,
    pub breathing_rate_avg: Option<f32>,
    /// 1 = paljon alle tavallisen … 6 = paljon yli tavallisen.
    pub nightly_recharge_status: Option<i16>,
    pub ans_charge: Option<f32>,
    pub ans_charge_status: Option<i16>,
    #[schema(value_type = Option<Object>)]
    pub hrv_samples: Option<Value>,
    #[schema(value_type = Option<Object>)]
    pub breathing_samples: Option<Value>,
}

/// Nightly Recharge -palautuminen aikaväliltä (oletus 30 päivää).
#[utoipa::path(
    get, path = "/recharge", tag = "data",
    params(RangeQuery),
    responses((status = 200, body = Vec<NightlyRecharge>), (status = 400, body = ErrorBody))
)]
async fn list_recharge(
    State(state): State<AppState>,
    _read: ReadAccess,
    Query(q): Query<RangeQuery>,
) -> ApiResult<Json<Vec<NightlyRecharge>>> {
    let (from, to) = q.resolve(30, MAX_RANGE_DAYS)?;
    let Some(account) = primary_account(&state.pool).await? else {
        return Ok(Json(vec![]));
    };
    let rows = sqlx::query_as!(
        NightlyRecharge,
        r#"SELECT date, heart_rate_avg, beat_to_beat_avg_ms, hrv_avg_ms, breathing_rate_avg,
                  nightly_recharge_status, ans_charge, ans_charge_status, hrv_samples,
                  breathing_samples
           FROM nightly_recharge
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
// Päiväaktiivisuus
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, ToSchema)]
pub struct DailyActivity {
    pub date: NaiveDate,
    pub active_duration_s: Option<i32>,
    pub inactive_duration_s: Option<i32>,
    /// Päivätavoitteen täyttymisprosentti.
    pub daily_activity_pct: Option<f32>,
    pub calories: Option<i32>,
    pub active_calories: Option<i32>,
    pub steps: Option<i32>,
    pub inactivity_alert_count: Option<i16>,
    pub distance_from_steps_m: Option<f32>,
}

/// Päiväaktiivisuus aikaväliltä (oletus 30 päivää).
#[utoipa::path(
    get, path = "/activity", tag = "data",
    params(RangeQuery),
    responses((status = 200, body = Vec<DailyActivity>), (status = 400, body = ErrorBody))
)]
async fn list_activity(
    State(state): State<AppState>,
    _read: ReadAccess,
    Query(q): Query<RangeQuery>,
) -> ApiResult<Json<Vec<DailyActivity>>> {
    let (from, to) = q.resolve(30, MAX_RANGE_DAYS)?;
    let Some(account) = primary_account(&state.pool).await? else {
        return Ok(Json(vec![]));
    };
    let rows = sqlx::query_as!(
        DailyActivity,
        r#"SELECT date, active_duration_s, inactive_duration_s, daily_activity_pct, calories,
                  active_calories, steps, inactivity_alert_count, distance_from_steps_m
           FROM daily_activity
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
// Cardio load
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, ToSchema)]
pub struct CardioLoad {
    pub date: NaiveDate,
    /// Esim. `PRODUCTIVE`, `MAINTAINING`, `OVERREACHING`.
    pub status: Option<String>,
    pub cardio_load: Option<f32>,
    pub strain: Option<f32>,
    pub tolerance: Option<f32>,
    pub cardio_load_ratio: Option<f32>,
    #[schema(value_type = Option<Object>)]
    pub cardio_load_level: Option<Value>,
}

/// Cardio load aikaväliltä (oletus 30 päivää).
#[utoipa::path(
    get, path = "/cardio-load", tag = "data",
    params(RangeQuery),
    responses((status = 200, body = Vec<CardioLoad>), (status = 400, body = ErrorBody))
)]
async fn list_cardio_load(
    State(state): State<AppState>,
    _read: ReadAccess,
    Query(q): Query<RangeQuery>,
) -> ApiResult<Json<Vec<CardioLoad>>> {
    let (from, to) = q.resolve(30, MAX_RANGE_DAYS)?;
    let Some(account) = primary_account(&state.pool).await? else {
        return Ok(Json(vec![]));
    };
    let rows = sqlx::query_as!(
        CardioLoad,
        r#"SELECT date, status, cardio_load, strain, tolerance, cardio_load_ratio, cardio_load_level
           FROM cardio_load
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
// Fyysiset tiedot
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, ToSchema)]
pub struct PhysicalInfo {
    pub date: NaiveDate,
    pub modified_at: DateTime<Utc>,
    pub weight_kg: Option<f32>,
    pub height_cm: Option<f32>,
    pub maximum_heart_rate: Option<i16>,
    pub resting_heart_rate: Option<i16>,
    pub aerobic_threshold: Option<i16>,
    pub anaerobic_threshold: Option<i16>,
    pub vo2_max: Option<i16>,
    pub weight_source: Option<String>,
    pub sleep_goal_s: Option<i32>,
}

/// Fyysisten tietojen aikasarja (paino, VO2max, leposyke) vanhimmasta uusimpaan.
#[utoipa::path(
    get, path = "/physical", tag = "data",
    responses((status = 200, body = Vec<PhysicalInfo>))
)]
async fn list_physical(
    State(state): State<AppState>,
    _read: ReadAccess,
) -> ApiResult<Json<Vec<PhysicalInfo>>> {
    let Some(account) = primary_account(&state.pool).await? else {
        return Ok(Json(vec![]));
    };
    let rows = sqlx::query_as!(
        PhysicalInfo,
        r#"SELECT date, modified_at, weight_kg, height_cm, maximum_heart_rate, resting_heart_rate,
                  aerobic_threshold, anaerobic_threshold, vo2_max, weight_source, sleep_goal_s
           FROM physical_info WHERE polar_account_id = $1 ORDER BY date"#,
        account
    )
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(rows))
}
