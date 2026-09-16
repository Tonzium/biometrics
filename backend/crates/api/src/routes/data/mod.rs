//! Julkiset lukureitit Polar-dataan. Kaikki ottavat `ReadAccess`-ekstraktorin,
//! joka päästää läpi ilman kirjautumista kun `PUBLIC_READ=true`.
//!
//! Vastauksista on jätetty pois kaikki tunnisteet (Polar-käyttäjä-id, laite-id,
//! tilin id) ja raaka JSON. Näytetään aina ensimmäiseksi rekisteröidyn
//! Polar-tilin data (sovelluksessa on yksi omistaja).

pub mod records;
pub mod summary;

use chrono::{Duration, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

use crate::{
    db,
    error::{ApiError, ApiResult},
};

/// Päivämääräväli kyselyparametreina. Oletus: viimeiset `default_days` päivää.
#[derive(Debug, Default, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct RangeQuery {
    /// Alkupäivä (YYYY-MM-DD), mukaan lukien.
    pub from: Option<NaiveDate>,
    /// Loppupäivä (YYYY-MM-DD), mukaan lukien. Oletus tänään.
    pub to: Option<NaiveDate>,
}

impl RangeQuery {
    pub fn resolve(&self, default_days: i64, max_days: i64) -> ApiResult<(NaiveDate, NaiveDate)> {
        let to = self.to.unwrap_or_else(|| Utc::now().date_naive());
        // Oletusalku lasketaan vain, jos `from` puuttuu, ja tarkistetulla
        // vähennyslaskulla. `to - Duration` panikoi, jos tulos alittaa
        // NaiveDaten alarajan, ja kyselyparametri `to=-262143-01-01` on juuri
        // se raja. Aiemmin `unwrap_or` laski erotuksen aina (myös kun `from`
        // oli annettu), joten pelkkä `to` riitti kaatamaan pyynnön ilman
        // kirjautumista: yhteys katkesi ja nginx vastasi 502.
        let from = match self.from {
            Some(from) => from,
            None => to
                .checked_sub_signed(Duration::days(default_days - 1))
                .ok_or_else(|| ApiError::BadRequest("`to` is out of range".into()))?,
        };
        if from > to {
            return Err(ApiError::BadRequest("`from` must not be after `to`".into()));
        }
        if (to - from).num_days() >= max_days {
            return Err(ApiError::BadRequest(format!(
                "date range must be at most {max_days} days"
            )));
        }
        Ok((from, to))
    }
}

/// Sivutettu vastaus.
#[derive(Debug, Serialize, ToSchema)]
pub struct Paged<T> {
    pub items: Vec<T>,
    pub page: u32,
    pub per_page: u32,
    pub total: i64,
}

pub const MAX_RANGE_DAYS: i64 = 366;

/// Piilotetaanko paino ja pituus tältä katselijalta.
pub fn hide_body_metrics(state: &crate::state::AppState, read: &crate::auth::ReadAccess) -> bool {
    !read.is_authenticated() && !state.config.public_body_metrics
}

/// Tili, jonka dataa julkiset reitit näyttävät. `None` = ei vielä yhdistetty.
pub async fn primary_account(pool: &PgPool) -> ApiResult<Option<Uuid>> {
    Ok(db::polar_accounts::first(pool).await?.map(|a| a.id))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_defaults_to_a_window_ending_today() {
        let (from, to) = RangeQuery::default().resolve(30, MAX_RANGE_DAYS).unwrap();
        assert_eq!(to, Utc::now().date_naive());
        assert_eq!((to - from).num_days(), 29);
    }

    /// Regressio: `to=-262143-01-01` (NaiveDaten alaraja) panikoi, koska
    /// oletusalku laskettiin `to - 29 päivää` ilman tarkistusta.
    #[test]
    fn resolve_rejects_to_below_date_range_instead_of_panicking() {
        let q = RangeQuery {
            from: None,
            to: Some(NaiveDate::MIN),
        };
        let err = q.resolve(30, MAX_RANGE_DAYS).unwrap_err();
        assert!(matches!(err, ApiError::BadRequest(_)), "{err:?}");
    }
}
