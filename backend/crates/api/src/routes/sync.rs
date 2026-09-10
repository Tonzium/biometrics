//! POST /api/sync        omistaja: käynnistää synkronoinnin heti ja odottaa tuloksen
//! GET  /api/sync/runs   omistaja: viimeisimmät ajot

use axum::{
    Json,
    extract::{Query, State},
};
use serde::Deserialize;
use utoipa::IntoParams;
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::{
    auth::CurrentUser,
    db::{self, sync_runs::SyncRunRecord},
    error::{ApiError, ApiResult, ErrorBody},
    state::AppState,
    sync::{self, SyncError, SyncReport, Trigger},
};

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(run_now))
        .routes(routes!(list_runs))
}

/// Omistaja: ajaa synkronoinnin heti ja palauttaa raportin.
#[utoipa::path(post, path = "/sync", tag = "sync",
    responses((status = 200, body = SyncReport), (status = 401, body = ErrorBody),
              (status = 404, body = ErrorBody), (status = 409, body = ErrorBody),
              (status = 503, body = ErrorBody)))]
async fn run_now(
    State(state): State<AppState>,
    current: CurrentUser,
) -> ApiResult<Json<SyncReport>> {
    current.require_owner()?;
    let account = db::polar_accounts::find_by_user(&state.pool, current.id)
        .await?
        .ok_or_else(|| ApiError::NotFound("no polar account linked".into()))?;

    let report = sync::run_sync(&state, &account, Trigger::Manual)
        .await
        .map_err(|e| match e {
            SyncError::AlreadyRunning => ApiError::Conflict(e.to_string()),
            SyncError::NotConfigured => ApiError::ServiceUnavailable(e.to_string()),
            SyncError::Token(inner) => ApiError::Internal(inner),
            SyncError::Db(inner) => inner.into(),
        })?;
    Ok(Json(report))
}

#[derive(Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct RunsQuery {
    /// Montako viimeisintä ajoa (1-200, oletus 20).
    limit: Option<i64>,
}

/// Omistaja: viimeisimmät synkronointiajot.
#[utoipa::path(get, path = "/sync/runs", tag = "sync", params(RunsQuery),
    responses((status = 200, body = Vec<SyncRunRecord>), (status = 401, body = ErrorBody)))]
async fn list_runs(
    State(state): State<AppState>,
    current: CurrentUser,
    Query(q): Query<RunsQuery>,
) -> ApiResult<Json<Vec<SyncRunRecord>>> {
    let Some(account) = db::polar_accounts::find_by_user(&state.pool, current.id).await? else {
        return Ok(Json(Vec::new()));
    };
    let limit = q.limit.unwrap_or(20).clamp(1, 200);
    let runs = db::sync_runs::list_recent(&state.pool, account.id, limit).await?;
    Ok(Json(runs))
}
