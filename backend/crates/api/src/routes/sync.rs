//! POST /api/sync        omistaja: käynnistää synkronoinnin heti ja odottaa tuloksen
//! GET  /api/sync/runs   omistaja: viimeisimmät ajot

use axum::{
    Json, Router,
    extract::{Query, State},
    routing::{get, post},
};
use serde::Deserialize;

use crate::{
    auth::CurrentUser,
    db::{self, sync_runs::SyncRunRecord},
    error::{ApiError, ApiResult},
    state::AppState,
    sync::{self, SyncError, SyncReport, Trigger},
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/sync", post(run_now))
        .route("/sync/runs", get(list_runs))
}

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

#[derive(Deserialize)]
pub struct RunsQuery {
    limit: Option<i64>,
}

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
