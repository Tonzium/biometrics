mod auth;
mod health;
mod polar;

use axum::Router;
use tower_http::trace::TraceLayer;

use crate::state::AppState;

/// Koko sovelluksen reititin. Kaikki reitit ovat `/api`-etuliitteen alla,
/// jotta nginx voi ohjata ne yksiselitteisesti backendille.
pub fn router(state: AppState) -> Router {
    let api = Router::new()
        .merge(health::router())
        .merge(auth::router())
        .merge(polar::router());

    Router::new()
        .nest("/api", api)
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
