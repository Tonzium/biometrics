//! Synkronointimoottori: hakee Polarista kaikki datatyypit ja upsertaa ne kantaan.
//!
//! - Yksi ajo kerrallaan (`AppState::sync_lock`); rinnakkainen yritys saa
//!   [`SyncError::AlreadyRunning`].
//! - Jokainen datatyyppi on oma askeleensa. Yhden askeleen virhe ei estä
//!   muita, paitsi Polarin 429 (rate limit), joka keskeyttää ajon.
//! - Jokainen ajo kirjataan `sync_runs`-tauluun tuloksineen.

pub mod scheduler;

use std::future::Future;

use chrono::{DateTime, Duration, Utc};
use polar_client::{
    PolarError,
    data::Batch,
    models::{CardioLoad, DailyActivity, Exercise, Fetched, NightlyRecharge, SleepNight},
};
use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    db::{self, polar_accounts::PolarAccountRecord},
    state::AppState,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Trigger {
    Initial,
    Manual,
    Scheduled,
}

impl Trigger {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Initial => "initial",
            Self::Manual => "manual",
            Self::Scheduled => "scheduled",
        }
    }
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct SyncCounts {
    pub exercises: u32,
    pub sleep_nights: u32,
    pub nightly_recharge: u32,
    pub daily_activity: u32,
    pub physical_info: u32,
    pub cardio_load: u32,
    /// Alkiot, joita ei voitu jäsentää tai tallentaa.
    pub skipped: u32,
}

impl SyncCounts {
    fn total(&self) -> u32 {
        self.exercises
            + self.sleep_nights
            + self.nightly_recharge
            + self.daily_activity
            + self.physical_info
            + self.cardio_load
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct SyncReport {
    pub run_id: i64,
    pub trigger: &'static str,
    /// `ok`, `partial` tai `failed`.
    pub status: &'static str,
    pub counts: SyncCounts,
    pub errors: Vec<String>,
    pub started_at: DateTime<Utc>,
    pub finished_at: DateTime<Utc>,
}

#[derive(Debug, thiserror::Error)]
pub enum SyncError {
    #[error("a sync is already running")]
    AlreadyRunning,
    #[error("Polar client credentials are not configured")]
    NotConfigured,
    #[error("stored Polar token could not be decrypted: {0}")]
    Token(anyhow::Error),
    #[error(transparent)]
    Db(#[from] sqlx::Error),
}

/// Ajaa täyden synkronoinnin yhdelle Polar-tilille.
pub async fn run_sync(
    state: &AppState,
    account: &PolarAccountRecord,
    trigger: Trigger,
) -> Result<SyncReport, SyncError> {
    let _guard = state
        .sync_lock
        .try_lock()
        .map_err(|_| SyncError::AlreadyRunning)?;
    let polar = state.polar.as_ref().ok_or(SyncError::NotConfigured)?;
    let token = state
        .cipher
        .decrypt(&account.access_token_enc)
        .map_err(SyncError::Token)?;

    let started_at = Utc::now();
    let run_id = db::sync_runs::start(&state.pool, account.id, trigger.as_str()).await?;
    tracing::info!(run_id, trigger = trigger.as_str(), "sync started");

    let pool = &state.pool;
    let account_id = account.id;
    let mut counts = SyncCounts::default();
    let mut errors = Vec::new();
    let mut aborted = false;

    // exercises
    if !aborted {
        let r = polar.exercises(&token).await;
        aborted = apply_batch(
            pool,
            account_id,
            "exercises",
            r,
            &mut counts.exercises,
            &mut counts.skipped,
            &mut errors,
        )
        .await;
    }
    if !aborted {
        let r = polar.sleep(&token).await;
        aborted = apply_batch(
            pool,
            account_id,
            "sleep",
            r,
            &mut counts.sleep_nights,
            &mut counts.skipped,
            &mut errors,
        )
        .await;
    }
    if !aborted {
        let r = polar.nightly_recharge(&token).await;
        aborted = apply_batch(
            pool,
            account_id,
            "nightly_recharge",
            r,
            &mut counts.nightly_recharge,
            &mut counts.skipped,
            &mut errors,
        )
        .await;
    }
    if !aborted {
        // Polar sallii enintään 28 päivän välin.
        let to = Utc::now().date_naive();
        let from = to - Duration::days(27);
        let r = polar.activities(&token, from, to).await;
        aborted = apply_batch(
            pool,
            account_id,
            "activities",
            r,
            &mut counts.daily_activity,
            &mut counts.skipped,
            &mut errors,
        )
        .await;
    }
    if !aborted {
        match polar.physical_info(&token).await {
            Ok(Some(item)) => {
                match db::polar_data::upsert_physical_info(pool, account_id, &item).await {
                    Ok(()) => counts.physical_info += 1,
                    Err(e) => {
                        counts.skipped += 1;
                        tracing::warn!(error = %e, "physical_info: upsert failed");
                    }
                }
            }
            Ok(None) => {}
            Err(e) => aborted = record_error("physical_info", e, &mut errors),
        }
    }
    if !aborted {
        let r = polar.cardio_load(&token).await;
        apply_batch(
            pool,
            account_id,
            "cardio_load",
            r,
            &mut counts.cardio_load,
            &mut counts.skipped,
            &mut errors,
        )
        .await;
    }

    let status = if errors.is_empty() {
        "ok"
    } else if counts.total() == 0 {
        "failed"
    } else {
        "partial"
    };
    let counts_json = serde_json::to_value(&counts).unwrap_or_default();
    let error_text = (!errors.is_empty()).then(|| errors.join("; "));
    db::sync_runs::finish(pool, run_id, status, &counts_json, error_text.as_deref()).await?;
    if status != "failed" {
        db::polar_accounts::set_last_sync_at(pool, account_id).await?;
    }

    let finished_at = Utc::now();
    tracing::info!(run_id, status, ?counts, "sync finished");
    Ok(SyncReport {
        run_id,
        trigger: trigger.as_str(),
        status,
        counts,
        errors,
        started_at,
        finished_at,
    })
}

/// Upsertaa erän alkiot. Palauttaa `true`, jos ajo pitää keskeyttää (429).
/// Tyyppi, jolle on olemassa upsert kantaan. Trait-pohjainen ratkaisu
/// sulkeuman sijaan, jotta futuret ovat varmasti `Send` (tokio::spawn).
trait Upsertable: Sized {
    fn upsert(
        pool: &PgPool,
        account_id: Uuid,
        item: &Fetched<Self>,
    ) -> impl Future<Output = anyhow::Result<()>> + Send;
}

macro_rules! upsertable {
    ($ty:ty => $f:path) => {
        impl Upsertable for $ty {
            fn upsert(
                pool: &PgPool,
                account_id: Uuid,
                item: &Fetched<Self>,
            ) -> impl Future<Output = anyhow::Result<()>> + Send {
                $f(pool, account_id, item)
            }
        }
    };
}

upsertable!(Exercise => db::polar_data::upsert_exercise);
upsertable!(SleepNight => db::polar_data::upsert_sleep);
upsertable!(NightlyRecharge => db::polar_data::upsert_recharge);
upsertable!(DailyActivity => db::polar_data::upsert_activity);
upsertable!(CardioLoad => db::polar_data::upsert_cardio_load);

async fn apply_batch<T: Upsertable>(
    pool: &PgPool,
    account_id: Uuid,
    name: &str,
    fetched: Result<Batch<T>, PolarError>,
    upserted: &mut u32,
    skipped: &mut u32,
    errors: &mut Vec<String>,
) -> bool {
    let batch = match fetched {
        Ok(b) => b,
        Err(e) => return record_error(name, e, errors),
    };
    *skipped += batch.skipped;
    for item in &batch.items {
        match T::upsert(pool, account_id, item).await {
            Ok(()) => *upserted += 1,
            Err(e) => {
                *skipped += 1;
                tracing::warn!(resource = name, error = %e, "upsert failed, item skipped");
            }
        }
    }
    tracing::debug!(
        resource = name,
        upserted,
        skipped = batch.skipped,
        "step done"
    );
    false
}

/// Kirjaa askeleen virheen. Palauttaa `true`, jos kyseessä on rate limit.
fn record_error(name: &str, err: PolarError, errors: &mut Vec<String>) -> bool {
    let rate_limited = matches!(err, PolarError::RateLimited { .. });
    tracing::warn!(resource = name, error = %err, "step failed");
    errors.push(format!("{name}: {err}"));
    rate_limited
}

/// Synkronoi kaikki yhdistetyt tilit peräkkäin. Käytetään ajastimessa.
pub async fn sync_all(state: &AppState, trigger: Trigger) {
    let accounts = match db::polar_accounts::list_all(&state.pool).await {
        Ok(a) => a,
        Err(e) => {
            tracing::error!(error = %e, "could not list polar accounts");
            return;
        }
    };
    if accounts.is_empty() {
        tracing::info!("no polar accounts linked; nothing to sync");
        return;
    }
    for account in &accounts {
        if let Err(e) = run_sync(state, account, trigger).await {
            tracing::error!(polar_user_id = account.polar_user_id, error = %e, "sync failed");
        }
    }
}
