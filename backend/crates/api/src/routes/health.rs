use axum::{Json, extract::State, http::StatusCode};
use serde::Serialize;
use utoipa::ToSchema;
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::state::AppState;

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(health))
        .routes(routes!(meta))
}

#[derive(Serialize, ToSchema)]
pub struct Health {
    /// `ok` tai `degraded`.
    status: &'static str,
    /// `up` tai `down`.
    database: &'static str,
    version: &'static str,
}

/// Palvelimen ja kantayhteyden tila. 503, jos kanta ei vastaa.
#[utoipa::path(get, path = "/health", tag = "system",
    responses((status = 200, body = Health), (status = 503, body = Health)))]
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

#[derive(Serialize, ToSchema)]
pub struct Meta {
    /// Saako dataa lukea ilman kirjautumista.
    public_read: bool,
    /// Onko Polar-tunnukset asetettu palvelimelle.
    polar_configured: bool,
    version: &'static str,
}

/// Julkiset asetukset, joita käyttöliittymä tarvitsee ennen kirjautumista.
#[utoipa::path(get, path = "/meta", tag = "system", responses((status = 200, body = Meta)))]
async fn meta(State(state): State<AppState>) -> Json<Meta> {
    Json(Meta {
        public_read: state.config.public_read,
        polar_configured: state.polar.is_some(),
        version: env!("CARGO_PKG_VERSION"),
    })
}
