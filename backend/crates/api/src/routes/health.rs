use axum::{Json, Router, extract::State, http::StatusCode, routing::get};
use serde::Serialize;

use crate::state::AppState;

#[derive(Serialize)]
pub struct Health {
    status: &'static str,
    database: &'static str,
    version: &'static str,
}

pub fn router() -> Router<AppState> {
    Router::new().route("/health", get(health))
}

/// GET /api/health – palvelimen ja kantayhteyden tila.
/// Palauttaa 503, jos kanta ei vastaa, jotta orkestrointi näkee vian.
async fn health(State(state): State<AppState>) -> (StatusCode, Json<Health>) {
    let db_ok = sqlx::query_scalar::<_, i32>("SELECT 1")
        .fetch_one(&state.pool)
        .await
        .is_ok();

    let body = Health {
        status: if db_ok { "ok" } else { "degraded" },
        database: if db_ok { "up" } else { "down" },
        version: env!("CARGO_PKG_VERSION"),
    };
    let code = if db_ok {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };
    (code, Json(body))
}
