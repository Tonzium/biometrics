//! Polar-tilin yhdistäminen OAuth2:lla. Kaikki reitit ovat omistajan reittejä.
//!
//! GET    /api/polar/status      yhdistetty vai ei, viimeisin synkronointi
//! GET    /api/polar/connect     ohjaa Polar Flow'n valtuutussivulle
//! GET    /api/polar/callback    Polarin paluuohjaus: koodi -> token -> kanta
//! DELETE /api/polar/disconnect  poistaa rekisteröinnin Polarista ja tokenin kannasta

use axum::{
    Json,
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Redirect, Response},
};
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::{DateTime, Utc};
use polar_client::{PolarClient, PolarError, users::RegisterOutcome};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use time::Duration;
use utoipa::{IntoParams, ToSchema};
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::{
    auth::CurrentUser,
    db,
    error::{ApiError, ApiResult, ErrorBody},
    state::AppState,
};

/// CSRF-tilan cookie OAuth-kierroksen ajaksi.
const STATE_COOKIE: &str = "pdh_oauth_state";
const STATE_TTL_MINUTES: i64 = 10;
/// Minne selain ohjataan kierroksen jälkeen.
const SETTINGS_PATH: &str = "/settings";

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(status))
        .routes(routes!(connect))
        .routes(routes!(callback))
        .routes(routes!(disconnect))
}

#[derive(Serialize, ToSchema)]
pub struct PolarStatus {
    /// Onko Polar-tunnukset asetettu palvelimelle.
    configured: bool,
    connected: bool,
    polar_user_id: Option<i64>,
    registered_at: Option<DateTime<Utc>>,
    last_sync_at: Option<DateTime<Utc>>,
}

/// Omistaja: Polar-tilin tila.
#[utoipa::path(get, path = "/polar/status", tag = "polar",
    responses((status = 200, body = PolarStatus), (status = 401, body = ErrorBody)))]
async fn status(
    State(state): State<AppState>,
    current: CurrentUser,
) -> ApiResult<Json<PolarStatus>> {
    let account = db::polar_accounts::find_by_user(&state.pool, current.id).await?;
    Ok(Json(PolarStatus {
        configured: state.polar.is_some(),
        connected: account.is_some(),
        polar_user_id: account.as_ref().map(|a| a.polar_user_id),
        registered_at: account.as_ref().map(|a| a.registered_at),
        last_sync_at: account.and_then(|a| a.last_sync_at),
    }))
}

fn polar_client(state: &AppState) -> ApiResult<&PolarClient> {
    state.polar.as_ref().ok_or_else(|| {
        ApiError::ServiceUnavailable("Polar client credentials are not configured".into())
    })
}

fn state_cookie(state: &AppState, value: String, ttl: Duration) -> Cookie<'static> {
    Cookie::build((STATE_COOKIE, value))
        .path("/api/polar")
        .http_only(true)
        .secure(state.config.cookie_secure)
        .same_site(SameSite::Lax)
        .max_age(ttl)
        .build()
}

/// Omistaja: aloittaa OAuth2-kierroksen. Ohjaa selaimen Polar Flow:n valtuutussivulle.
#[utoipa::path(get, path = "/polar/connect", tag = "polar",
    responses((status = 303, description = "Redirect Polariin"), (status = 401, body = ErrorBody),
              (status = 403, body = ErrorBody), (status = 503, body = ErrorBody)))]
async fn connect(
    State(state): State<AppState>,
    current: CurrentUser,
    jar: CookieJar,
) -> ApiResult<(CookieJar, Redirect)> {
    current.require_owner()?;
    let polar = polar_client(&state)?;

    let mut random = [0u8; 32];
    rand::rng().fill_bytes(&mut random);
    let oauth_state = URL_SAFE_NO_PAD.encode(random);

    let url = polar.authorization_url(&oauth_state);
    let jar = jar.add(state_cookie(
        &state,
        oauth_state,
        Duration::minutes(STATE_TTL_MINUTES),
    ));
    Ok((jar, Redirect::to(&url)))
}

#[derive(Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct CallbackQuery {
    code: Option<String>,
    state: Option<String>,
    /// Polar asettaa tämän, jos käyttäjä kieltäytyy.
    error: Option<String>,
}

/// Polarin paluuohjaus. Tarkistaa `state`-cookien, vaihtaa koodin tokeniin ja tallentaa tilin.
/// Ohjaa lopuksi osoitteeseen `/settings?polar=connected|denied|error`.
#[utoipa::path(get, path = "/polar/callback", tag = "polar", params(CallbackQuery),
    responses((status = 303, description = "Redirect asetussivulle"), (status = 400, body = ErrorBody),
              (status = 401, body = ErrorBody)))]
async fn callback(
    State(state): State<AppState>,
    current: CurrentUser,
    jar: CookieJar,
    Query(query): Query<CallbackQuery>,
) -> ApiResult<Response> {
    current.require_owner()?;
    let polar = polar_client(&state)?;

    // Tilacookie poistetaan aina, onnistui kierros tai ei.
    let expected_state = jar.get(STATE_COOKIE).map(|c| c.value().to_owned());
    let jar = jar.add(state_cookie(&state, String::new(), Duration::ZERO));

    if let Some(err) = query.error {
        tracing::info!(error = %err, "polar authorization denied");
        return Ok((jar, Redirect::to(&format!("{SETTINGS_PATH}?polar=denied"))).into_response());
    }

    let (Some(code), Some(returned_state)) = (query.code, query.state) else {
        return Err(ApiError::BadRequest("missing code or state".into()));
    };
    if expected_state.as_deref() != Some(returned_state.as_str()) {
        tracing::warn!("oauth state mismatch");
        return Err(ApiError::BadRequest("oauth state mismatch".into()));
    }

    let token = match polar.exchange_code(&code).await {
        Ok(t) => t,
        Err(e) => {
            tracing::error!(error = %e, "polar token exchange failed");
            return Ok((jar, Redirect::to(&format!("{SETTINGS_PATH}?polar=error"))).into_response());
        }
    };

    // member-id on oma tunnisteemme Polarin päässä; käyttäjän uuid on yksikäsitteinen.
    let member_id = current.id.to_string();
    match polar.register_user(&token.access_token, &member_id).await {
        Ok(RegisterOutcome::Registered(user)) => {
            tracing::info!(polar_user_id = user.polar_user_id, "registered polar user");
        }
        Ok(RegisterOutcome::AlreadyRegistered) => {
            tracing::info!(
                polar_user_id = token.x_user_id,
                "polar user already registered"
            );
        }
        Err(e) => {
            tracing::error!(error = %e, "polar user registration failed");
            return Ok((jar, Redirect::to(&format!("{SETTINGS_PATH}?polar=error"))).into_response());
        }
    }

    let token_enc = state.cipher.encrypt(&token.access_token)?;
    let polar_user_id = i64::try_from(token.x_user_id)
        .map_err(|_| ApiError::Internal(anyhow::anyhow!("polar user id out of range")))?;
    db::polar_accounts::upsert_for_user(
        &state.pool,
        current.id,
        polar_user_id,
        &token_enc,
        &member_id,
    )
    .await
    .map_err(|e| match e {
        sqlx::Error::Database(ref db) if db.is_unique_violation() => {
            ApiError::Conflict("this Polar account is already linked to another user".into())
        }
        other => other.into(),
    })?;

    tracing::info!(user = %current.email, polar_user_id, "polar account connected");
    Ok((
        jar,
        Redirect::to(&format!("{SETTINGS_PATH}?polar=connected")),
    )
        .into_response())
}

/// Omistaja: poistaa rekisteröinnin Polarista ja tokenin kannasta.
#[utoipa::path(delete, path = "/polar/disconnect", tag = "polar",
    responses((status = 204), (status = 401, body = ErrorBody), (status = 404, body = ErrorBody)))]
async fn disconnect(State(state): State<AppState>, current: CurrentUser) -> ApiResult<StatusCode> {
    current.require_owner()?;
    let Some(account) = db::polar_accounts::find_by_user(&state.pool, current.id).await? else {
        return Err(ApiError::NotFound("no polar account linked".into()));
    };

    // Poistetaan rekisteröinti Polarista, jos tunnukset ovat käytössä.
    // Epäonnistuminen ei estä paikallista poistoa, mutta se lokitetaan.
    if let Some(polar) = &state.polar {
        match state.cipher.decrypt(&account.access_token_enc) {
            Ok(token) => {
                let user_id = u64::try_from(account.polar_user_id).unwrap_or_default();
                match polar.delete_user(&token, user_id).await {
                    Ok(()) => tracing::info!(polar_user_id = user_id, "deregistered from polar"),
                    Err(PolarError::Status {
                        status: 401 | 403, ..
                    }) => {
                        tracing::warn!("polar token no longer valid; removing locally only")
                    }
                    Err(e) => {
                        tracing::warn!(error = %e, "polar deregistration failed; removing locally")
                    }
                }
            }
            Err(e) => tracing::warn!(error = %e, "could not decrypt polar token; removing locally"),
        }
    }

    db::polar_accounts::delete_by_user(&state.pool, current.id).await?;
    Ok(StatusCode::NO_CONTENT)
}
