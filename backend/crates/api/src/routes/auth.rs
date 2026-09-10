//! POST /api/auth/login, POST /api/auth/logout, GET /api/auth/me

use axum::{
    Json, Router,
    extract::State,
    routing::{get, post},
};
use axum_extra::extract::cookie::CookieJar;
use domain::User;
use serde::Deserialize;

use crate::{
    auth::{self, CurrentUser, jwt, password},
    db,
    error::{ApiError, ApiResult},
    state::AppState,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/auth/login", post(login))
        .route("/auth/logout", post(logout))
        .route("/auth/me", get(me))
}

#[derive(Deserialize)]
pub struct LoginRequest {
    email: String,
    password: String,
}

/// Onnistuessa asettaa istuntocookien ja palauttaa käyttäjän.
/// Epäonnistuessa 401 ilman erottelua "väärä sähköposti" / "väärä salasana".
async fn login(
    State(state): State<AppState>,
    jar: CookieJar,
    Json(req): Json<LoginRequest>,
) -> ApiResult<(CookieJar, Json<User>)> {
    let email = req.email.trim().to_lowercase();
    if email.is_empty() || req.password.is_empty() {
        return Err(ApiError::BadRequest(
            "email and password are required".into(),
        ));
    }

    let record = db::users::find_by_email(&state.pool, &email).await?;
    let stored_hash = record.as_ref().map(|r| r.password_hash.clone());

    // Verify ajetaan aina, myös tuntemattomalle käyttäjälle (ajoituspuolustus).
    let ok = password::verify(req.password, stored_hash).await?;
    let Some(record) = record.filter(|_| ok) else {
        tracing::info!(%email, "failed login attempt");
        return Err(ApiError::Unauthorized);
    };

    let user = record.into_user();
    let token = jwt::issue(&state.config, &user)?;
    let jar = jar.add(auth::session_cookie(&state.config, token));
    tracing::info!(user = %user.email, "login");
    Ok((jar, Json(user)))
}

async fn logout(
    State(state): State<AppState>,
    jar: CookieJar,
) -> (CookieJar, Json<serde_json::Value>) {
    // `add` eikä `remove`: remove lähettäisi poistocookien vain, jos pyynnössä
    // oli sama cookie. Max-Age=0 poistaa sen selaimesta joka tapauksessa.
    let jar = jar.add(auth::removal_cookie(&state.config));
    (jar, Json(serde_json::json!({ "ok": true })))
}

/// Palauttaa kirjautuneen käyttäjän tuoreet tiedot kannasta.
/// Jos käyttäjä on poistettu tokenin myöntämisen jälkeen, 401.
async fn me(State(state): State<AppState>, current: CurrentUser) -> ApiResult<Json<User>> {
    let record = db::users::find_by_id(&state.pool, current.id)
        .await?
        .ok_or(ApiError::Unauthorized)?;
    Ok(Json(record.into_user()))
}
