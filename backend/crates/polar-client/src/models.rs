//! Polar AccessLink v3:n vastausrakenteet.
//!
//! Kenttänimet ja tyypit on tarkistettu `docs/reference/polar-accesslink-swagger.yaml`
//! -tiedostosta. Lähes kaikki kentät ovat `Option`, koska Polar jättää
//! puuttuvat arvot pois vastauksesta (esim. `distance` sisäharjoituksissa).

use chrono::{DateTime, NaiveDate, NaiveDateTime, Utc};
use serde::Deserialize;
use serde_json::Value;

/// Tyypitetty vastausolio yhdessä alkuperäisen JSON:n kanssa.
#[derive(Debug, Clone)]
pub struct Fetched<T> {
    pub data: T,
    pub raw: Value,
}

#[derive(Debug, Clone, Deserialize)]
pub struct HeartRate {
    #[serde(default)]
    pub average: Option<i32>,
    #[serde(default)]
    pub maximum: Option<i32>,
}

/// `GET /v3/exercises` -listan alkio (hash-id-muoto).
#[derive(Debug, Clone, Deserialize)]
pub struct Exercise {
    pub id: String,
    #[serde(default)]
    pub upload_time: Option<DateTime<Utc>>,
    /// Paikallinen aika ilman vyöhykettä.
    pub start_time: NaiveDateTime,
    /// Minuutteja UTC:stä (esim. 180 = UTC+3).
    #[serde(default)]
    pub start_time_utc_offset: Option<i32>,
    /// ISO 8601 -kesto, ks. [`crate::duration`].
    pub duration: String,
    #[serde(default)]
    pub calories: Option<i32>,
    #[serde(default)]
    pub distance: Option<f64>,
    #[serde(default)]
    pub heart_rate: Option<HeartRate>,
    #[serde(default)]
    pub training_load: Option<f64>,
    pub sport: String,
    #[serde(default)]
    pub has_route: bool,
    #[serde(default)]
    pub detailed_sport_info: Option<String>,
    #[serde(default)]
    pub device: Option<String>,
    #[serde(default)]
    pub device_id: Option<String>,
    #[serde(default, rename = "running-index")]
    pub running_index: Option<i32>,
    #[serde(default)]
    pub fat_percentage: Option<i32>,
    #[serde(default)]
    pub carbohydrate_percentage: Option<i32>,
    #[serde(default)]
    pub protein_percentage: Option<i32>,
    #[serde(default)]
    pub heart_rate_zones: Option<Value>,
    #[serde(default)]
    pub training_load_pro: Option<Value>,
}

impl Exercise {
    /// Alkuhetki UTC:nä: paikallinen aika miinus offset.
    pub fn start_time_utc(&self) -> DateTime<Utc> {
        let offset = chrono::Duration::minutes(i64::from(self.start_time_utc_offset.unwrap_or(0)));
        DateTime::<Utc>::from_naive_utc_and_offset(self.start_time - offset, Utc)
    }
}

/// `GET /v3/users/sleep` -> `{ "nights": [...] }`
#[derive(Debug, Clone, Deserialize)]
pub struct SleepNight {
    pub date: NaiveDate,
    pub sleep_start_time: DateTime<Utc>,
    pub sleep_end_time: DateTime<Utc>,
    #[serde(default)]
    pub device_id: Option<String>,
    #[serde(default)]
    pub continuity: Option<f64>,
    #[serde(default)]
    pub continuity_class: Option<i32>,
    #[serde(default)]
    pub light_sleep: Option<i32>,
    #[serde(default)]
    pub deep_sleep: Option<i32>,
    #[serde(default)]
    pub rem_sleep: Option<i32>,
    #[serde(default)]
    pub unrecognized_sleep_stage: Option<i32>,
    #[serde(default)]
    pub sleep_score: Option<i32>,
    #[serde(default)]
    pub total_interruption_duration: Option<i32>,
    #[serde(default)]
    pub sleep_charge: Option<i32>,
    #[serde(default)]
    pub sleep_goal: Option<i32>,
    #[serde(default)]
    pub sleep_rating: Option<i32>,
    #[serde(default)]
    pub short_interruption_duration: Option<i32>,
    #[serde(default)]
    pub long_interruption_duration: Option<i32>,
    #[serde(default)]
    pub sleep_cycles: Option<i32>,
    #[serde(default)]
    pub group_duration_score: Option<f64>,
    #[serde(default)]
    pub group_solidity_score: Option<f64>,
    #[serde(default)]
    pub group_regeneration_score: Option<f64>,
    #[serde(default)]
    pub hypnogram: Option<Value>,
    #[serde(default)]
    pub heart_rate_samples: Option<Value>,
}

/// `GET /v3/users/nightly-recharge` -> `{ "recharges": [...] }`
#[derive(Debug, Clone, Deserialize)]
pub struct NightlyRecharge {
    pub date: NaiveDate,
    #[serde(default)]
    pub heart_rate_avg: Option<i32>,
    #[serde(default)]
    pub beat_to_beat_avg: Option<i32>,
    #[serde(default)]
    pub heart_rate_variability_avg: Option<i32>,
    #[serde(default)]
    pub breathing_rate_avg: Option<f64>,
    #[serde(default)]
    pub nightly_recharge_status: Option<i32>,
    #[serde(default)]
    pub ans_charge: Option<f64>,
    #[serde(default)]
    pub ans_charge_status: Option<i32>,
    #[serde(default)]
    pub hrv_samples: Option<Value>,
    #[serde(default)]
    pub breathing_samples: Option<Value>,
}

/// `GET /v3/users/activities?from&to` -> `[ ... ]`
#[derive(Debug, Clone, Deserialize)]
pub struct DailyActivity {
    pub start_time: NaiveDateTime,
    #[serde(default)]
    pub end_time: Option<NaiveDateTime>,
    #[serde(default)]
    pub active_duration: Option<String>,
    #[serde(default)]
    pub inactive_duration: Option<String>,
    #[serde(default)]
    pub daily_activity: Option<f64>,
    #[serde(default)]
    pub calories: Option<i32>,
    #[serde(default)]
    pub active_calories: Option<i32>,
    #[serde(default)]
    pub steps: Option<i32>,
    #[serde(default)]
    pub inactivity_alert_count: Option<i32>,
    #[serde(default)]
    pub distance_from_steps: Option<f64>,
    #[serde(default)]
    pub samples: Option<Value>,
}

impl DailyActivity {
    /// Päivä, jota rivi koskee. `end_time` on Polarilla päivän viimeinen
    /// sekunti (23:59:59), joten sen päivämäärä on luotettavin.
    pub fn date(&self) -> NaiveDate {
        self.end_time.unwrap_or(self.start_time).date()
    }
}

/// `GET /v3/users/physical-info` -> yksi olio (nykytila)
#[derive(Debug, Clone, Deserialize)]
pub struct PhysicalInfo {
    #[serde(default)]
    pub created: Option<DateTime<Utc>>,
    pub modified: DateTime<Utc>,
    #[serde(default)]
    pub weight: Option<f64>,
    #[serde(default)]
    pub height: Option<f64>,
    #[serde(default)]
    pub maximum_heart_rate: Option<i32>,
    #[serde(default)]
    pub resting_heart_rate: Option<i32>,
    #[serde(default)]
    pub aerobic_threshold: Option<i32>,
    #[serde(default)]
    pub anaerobic_threshold: Option<i32>,
    #[serde(default)]
    pub vo2_max: Option<i32>,
    #[serde(default)]
    pub weight_source: Option<String>,
    #[serde(default)]
    pub training_background: Option<String>,
    #[serde(default)]
    pub typical_day: Option<String>,
    #[serde(default)]
    pub sleep_goal: Option<String>,
}

/// `GET /v3/users/cardio-load` -> `[ ... ]`
#[derive(Debug, Clone, Deserialize)]
pub struct CardioLoad {
    pub date: NaiveDate,
    #[serde(default)]
    pub cardio_load_status: Option<String>,
    #[serde(default)]
    pub cardio_load: Option<f64>,
    #[serde(default)]
    pub strain: Option<f64>,
    #[serde(default)]
    pub tolerance: Option<f64>,
    #[serde(default)]
    pub cardio_load_ratio: Option<f64>,
    #[serde(default)]
    pub cardio_load_level: Option<Value>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exercise_start_time_converts_to_utc() {
        let ex: Exercise = serde_json::from_value(serde_json::json!({
            "id": "ABC", "start_time": "2026-09-01T10:00:00", "start_time_utc_offset": 180,
            "duration": "PT1H", "sport": "RUNNING"
        }))
        .unwrap();
        assert_eq!(
            ex.start_time_utc().to_rfc3339(),
            "2026-09-01T07:00:00+00:00"
        );
        assert!(!ex.has_route);
        assert!(ex.distance.is_none());
    }

    #[test]
    fn sleep_times_with_offset_parse() {
        let n: SleepNight = serde_json::from_value(serde_json::json!({
            "date": "2026-09-02",
            "sleep_start_time": "2026-09-01T23:10:00+03:00",
            "sleep_end_time": "2026-09-02T06:40:00+03:00",
            "sleep_score": 81
        }))
        .unwrap();
        assert_eq!(n.sleep_start_time.to_rfc3339(), "2026-09-01T20:10:00+00:00");
        assert_eq!(n.sleep_score, Some(81));
    }

    #[test]
    fn activity_date_comes_from_end_time() {
        let a: DailyActivity = serde_json::from_value(serde_json::json!({
            "start_time": "2026-09-01T21:00:00", "end_time": "2026-09-02T23:59:59", "steps": 10
        }))
        .unwrap();
        assert_eq!(a.date(), NaiveDate::from_ymd_opt(2026, 9, 2).unwrap());
    }
}
