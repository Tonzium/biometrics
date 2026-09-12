//! POST /api/auth/login, POST /api/auth/logout, GET /api/auth/me

use axum::{Json, extract::State};
use axum_extra::extract::cookie::CookieJar;
use domain::User;
use serde::Deserialize;
use utoipa::ToSchema;
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::{
    auth::{self, CurrentUser, LOGIN_RETRY_AFTER_SECS, jwt, password},
    db,
    error::{ApiError, ApiResult, ErrorBody},
    state::AppState,
};

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(login))
        .routes(routes!(logout))
        .routes(routes!(me))
}

#[derive(Deserialize, ToSchema)]
pub struct LoginRequest {
    email: String,
    password: String,
}

/// Kirjautuminen. Onnistuessa asettaa `pdh_session`-cookien ja palauttaa käyttäjän.
/// Epäonnistuessa 401 ilman erottelua "väärä sähköposti" / "väärä salasana".
/// Jos salasanatarkistuksia on jo käynnissä rajan verran, vastaus on 429.
#[utoipa::path(post, path = "/auth/login", tag = "auth", request_body = LoginRequest,
    responses((status = 200, body = User), (status = 400, body = ErrorBody),
              (status = 401, body = ErrorBody), (status = 429, body = ErrorBody)))]
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

    // Rinnakkaisuusraja otetaan ennen kantakyselyä: hylätty pyyntö ei varaa
    // yhteyttä poolista eikä käynnistä argon2-laskentaa. Permit annetaan
    // `verify`lle, joka vapauttaa sen vasta laskennan päätyttyä. Lokiriviin ei
    // oteta mitään pyynnöstä, jotta tulva ei voi kirjoittaa omaa tekstiään.
    let Ok(permit) = state.login_limit.clone().try_acquire_owned() else {
        tracing::warn!("login rejected: concurrency limit reached");
        return Err(ApiError::TooManyRequests {
            message: "too many login attempts, try again shortly".into(),
            retry_after_secs: Some(LOGIN_RETRY_AFTER_SECS),
        });
    };

    let record = db::users::find_by_email(&state.pool, &email).await?;
    let stored_hash = record.as_ref().map(|r| r.password_hash.clone());

    // Verify ajetaan aina, myös tuntemattomalle käyttäjälle (ajoituspuolustus).
    let ok = password::verify(req.password, stored_hash, permit).await?;
    let Some(record) = record.filter(|_| ok) else {
        tracing::info!(email = ?email, "failed login attempt");
        return Err(ApiError::Unauthorized);
    };

    let token_version = record.token_version;
    let user = record.into_user();
    let token = jwt::issue(&state.config, &user, token_version)?;
    let jar = jar.add(auth::session_cookie(&state.config, token));
    tracing::info!(user = %user.email, "login");
    Ok((jar, Json(user)))
}

/// Poistaa istuntocookien.
#[utoipa::path(post, path = "/auth/logout", tag = "auth", responses((status = 200)))]
async fn logout(
    State(state): State<AppState>,
    jar: CookieJar,
) -> (CookieJar, Json<serde_json::Value>) {
    // `add` eikä `remove`: remove lähettäisi poistocookien vain, jos pyynnössä
    // oli sama cookie. Max-Age=0 poistaa sen selaimesta joka tapauksessa.
    let jar = jar.add(auth::removal_cookie(&state.config));
    (jar, Json(serde_json::json!({ "ok": true })))
}

/// Kirjautuneen käyttäjän tiedot. 401, jos istuntoa ei ole tai käyttäjä on poistettu.
#[utoipa::path(get, path = "/auth/me", tag = "auth",
    responses((status = 200, body = User), (status = 401, body = ErrorBody)))]
async fn me(State(state): State<AppState>, current: CurrentUser) -> ApiResult<Json<User>> {
    let record = db::users::find_by_id(&state.pool, current.id)
        .await?
        .ok_or(ApiError::Unauthorized)?;
    Ok(Json(record.into_user()))
}
