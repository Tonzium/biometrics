//! `CurrentUser`-ekstraktori: reitti, joka ottaa tämän parametrina,
//! on automaattisesti suojattu. Puuttuva tai virheellinen istunto
//! palauttaa 401 ennen kuin reitin koodi ajetaan.

use axum::{extract::FromRequestParts, http::request::Parts};
use axum_extra::extract::cookie::CookieJar;
use domain::Role;
use uuid::Uuid;

use super::{SESSION_COOKIE, jwt};
use crate::{error::ApiError, state::AppState};

#[derive(Debug, Clone)]
pub struct CurrentUser {
    pub id: Uuid,
    pub email: String,
    pub role: Role,
}

impl CurrentUser {
    /// Palauttaa 403, jos käyttäjä ei ole omistaja.
    pub fn require_owner(&self) -> Result<(), ApiError> {
        if self.role == Role::Owner {
            Ok(())
        } else {
            Err(ApiError::Forbidden)
        }
    }
}

impl FromRequestParts<AppState> for CurrentUser {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let jar = CookieJar::from_headers(&parts.headers);
        let token = jar
            .get(SESSION_COOKIE)
            .map(|c| c.value().to_owned())
            .ok_or(ApiError::Unauthorized)?;

        let claims = jwt::verify(&state.config, &token).map_err(|e| {
            tracing::debug!(error = %e, "rejected session token");
            ApiError::Unauthorized
        })?;

        Ok(Self {
            id: claims.sub,
            email: claims.email,
            role: claims.role,
        })
    }
}

/// Lukuoikeus dataan. Jos `PUBLIC_READ=true` (oletus, näyteikkuna), kuka
/// tahansa saa lukea; muuten vaaditaan kirjautunut käyttäjä.
///
/// `user` on `Some`, jos pyynnössä oli kelvollinen istunto, myös julkisessa
/// tilassa. Reitit käyttävät sitä päättääkseen, mitkä kentät näytetään
/// (esim. paino vain kirjautuneille).
#[derive(Debug, Clone)]
pub struct ReadAccess {
    pub user: Option<CurrentUser>,
}

impl ReadAccess {
    pub fn is_authenticated(&self) -> bool {
        self.user.is_some()
    }
}

impl FromRequestParts<AppState> for ReadAccess {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let user = CurrentUser::from_request_parts(parts, state).await;
        match user {
            Ok(user) => Ok(Self { user: Some(user) }),
            Err(_) if state.config.public_read => Ok(Self { user: None }),
            Err(e) => Err(e),
        }
    }
}
