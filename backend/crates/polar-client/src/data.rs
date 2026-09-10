//! Datan haku AccessLinkistä. Jokainen metodi palauttaa listan
//! [`Fetched`]-olioita, joissa on sekä tyypitetty data että raaka JSON.
//!
//! Yksittäinen alkio, joka ei jäsenny, ohitetaan varoituksella eikä kaada
//! koko hakua: Polarin skeema voi laajentua, ja on parempi tallentaa
//! 29 yötä kuin ei yhtään.

use chrono::NaiveDate;
use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::{
    PolarClient, PolarError, http,
    models::{
        CardioLoad, DailyActivity, Exercise, Fetched, NightlyRecharge, PhysicalInfo, SleepNight,
    },
};

/// Haun tulos: jäsentyneet alkiot ja ohitettujen määrä.
#[derive(Debug, Default)]
pub struct Batch<T> {
    pub items: Vec<Fetched<T>>,
    pub skipped: u32,
}

impl PolarClient {
    /// Viimeisten ~30 päivän harjoitukset sykevyöhykkeineen.
    pub async fn exercises(&self, token: &str) -> Result<Batch<Exercise>, PolarError> {
        let value = self
            .get_json(token, "/exercises", &[("zones", "true")])
            .await?;
        Ok(parse_batch("exercises", array_at(value, None)?))
    }

    /// Viimeisten 28 päivän yöt.
    pub async fn sleep(&self, token: &str) -> Result<Batch<SleepNight>, PolarError> {
        let value = self.get_json(token, "/users/sleep", &[]).await?;
        Ok(parse_batch("sleep", array_at(value, Some("nights"))?))
    }

    pub async fn nightly_recharge(
        &self,
        token: &str,
    ) -> Result<Batch<NightlyRecharge>, PolarError> {
        let value = self.get_json(token, "/users/nightly-recharge", &[]).await?;
        Ok(parse_batch(
            "nightly_recharge",
            array_at(value, Some("recharges"))?,
        ))
    }

    /// Päiväaktiivisuus väliltä `from..=to`. Polar sallii enintään 28 päivää.
    pub async fn activities(
        &self,
        token: &str,
        from: NaiveDate,
        to: NaiveDate,
    ) -> Result<Batch<DailyActivity>, PolarError> {
        let value = self
            .get_json(
                token,
                "/users/activities",
                &[("from", &from.to_string()), ("to", &to.to_string())],
            )
            .await?;
        Ok(parse_batch("activities", array_at(value, None)?))
    }

    /// Nykyiset fyysiset tiedot (yksi olio).
    pub async fn physical_info(
        &self,
        token: &str,
    ) -> Result<Option<Fetched<PhysicalInfo>>, PolarError> {
        let value = self.get_json(token, "/users/physical-info", &[]).await?;
        if value.is_null() {
            return Ok(None);
        }
        match serde_json::from_value::<PhysicalInfo>(value.clone()) {
            Ok(data) => Ok(Some(Fetched { data, raw: value })),
            Err(e) => Err(PolarError::Decode(format!("physical-info: {e}"))),
        }
    }

    /// Viimeisten 28 päivän cardio load.
    pub async fn cardio_load(&self, token: &str) -> Result<Batch<CardioLoad>, PolarError> {
        let value = self.get_json(token, "/users/cardio-load", &[]).await?;
        Ok(parse_batch("cardio_load", array_at(value, None)?))
    }

    async fn get_json(
        &self,
        token: &str,
        path: &str,
        query: &[(&str, &str)],
    ) -> Result<Value, PolarError> {
        let request = http::bearer(self.http.get(self.api_url(path)), token).query(query);
        let response = http::check(request.send().await?).await?;
        // 204 No Content = ei dataa (Polar käyttää tätä tyhjille listoille).
        if response.status().as_u16() == 204 {
            return Ok(Value::Null);
        }
        http::json(response).await
    }
}

/// Poimii taulukon joko juuresta tai annetun avaimen takaa. `null` = tyhjä.
fn array_at(value: Value, key: Option<&str>) -> Result<Vec<Value>, PolarError> {
    let inner = match (value, key) {
        (Value::Null, _) => return Ok(Vec::new()),
        (Value::Object(mut map), Some(k)) => map.remove(k).unwrap_or(Value::Null),
        (v, _) => v,
    };
    match inner {
        Value::Array(items) => Ok(items),
        Value::Null => Ok(Vec::new()),
        other => Err(PolarError::Decode(format!(
            "expected array{}, got {}",
            key.map(|k| format!(" under \"{k}\"")).unwrap_or_default(),
            kind(&other)
        ))),
    }
}

fn parse_batch<T: DeserializeOwned>(what: &str, values: Vec<Value>) -> Batch<T> {
    let mut batch = Batch {
        items: Vec::with_capacity(values.len()),
        skipped: 0,
    };
    for raw in values {
        match serde_json::from_value::<T>(raw.clone()) {
            Ok(data) => batch.items.push(Fetched { data, raw }),
            Err(e) => {
                batch.skipped += 1;
                tracing::warn!(resource = what, error = %e, "skipping unparseable item");
            }
        }
    }
    batch
}

fn kind(v: &Value) -> &'static str {
    match v {
        Value::Null => "null",
        Value::Bool(_) => "bool",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

#[cfg(test)]
mod tests {
    use wiremock::{
        Mock, MockServer, ResponseTemplate,
        matchers::{bearer_token, method, path, query_param},
    };

    use super::*;
    use crate::PolarConfig;

    async fn client(server: &MockServer) -> PolarClient {
        PolarClient::new(PolarConfig::for_base_url(&server.uri())).unwrap()
    }

    #[tokio::test]
    async fn exercises_requests_zones_and_skips_bad_items() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/v3/exercises"))
            .and(query_param("zones", "true"))
            .and(bearer_token("tok"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
                { "id": "A1", "start_time": "2026-09-01T10:00:00", "start_time_utc_offset": 180,
                  "duration": "PT1H", "sport": "RUNNING", "distance": 10500.5,
                  "heart_rate": { "average": 150, "maximum": 178 } },
                { "id": "BROKEN" }
            ])))
            .mount(&server)
            .await;

        let batch = client(&server).await.exercises("tok").await.unwrap();
        assert_eq!(batch.items.len(), 1);
        assert_eq!(batch.skipped, 1);
        assert_eq!(
            batch.items[0].data.heart_rate.as_ref().unwrap().maximum,
            Some(178)
        );
        assert_eq!(batch.items[0].raw["distance"], 10500.5);
    }

    #[tokio::test]
    async fn sleep_unwraps_nights_and_204_is_empty() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/v3/users/sleep"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "nights": [{ "date": "2026-09-02",
                             "sleep_start_time": "2026-09-01T23:10:00+03:00",
                             "sleep_end_time": "2026-09-02T06:40:00+03:00" }]
            })))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/v3/users/nightly-recharge"))
            .respond_with(ResponseTemplate::new(204))
            .mount(&server)
            .await;

        let c = client(&server).await;
        assert_eq!(c.sleep("tok").await.unwrap().items.len(), 1);
        assert!(c.nightly_recharge("tok").await.unwrap().items.is_empty());
    }

    #[tokio::test]
    async fn activities_sends_date_range() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/v3/users/activities"))
            .and(query_param("from", "2026-08-05"))
            .and(query_param("to", "2026-09-01"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
                { "start_time": "2026-09-01T00:00:00", "end_time": "2026-09-01T23:59:59", "steps": 8823 }
            ])))
            .expect(1)
            .mount(&server)
            .await;

        let from = NaiveDate::from_ymd_opt(2026, 8, 5).unwrap();
        let to = NaiveDate::from_ymd_opt(2026, 9, 1).unwrap();
        let batch = client(&server)
            .await
            .activities("tok", from, to)
            .await
            .unwrap();
        assert_eq!(batch.items[0].data.steps, Some(8823));
    }

    #[tokio::test]
    async fn wrong_shape_is_a_decode_error() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/v3/users/cardio-load"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"oops": 1})))
            .mount(&server)
            .await;
        let err = client(&server).await.cardio_load("tok").await.unwrap_err();
        assert!(matches!(err, PolarError::Decode(_)));
    }
}
