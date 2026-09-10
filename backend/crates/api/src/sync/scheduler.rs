//! Ajastettu synkronointi taustatehtävänä.
//!
//! Ensimmäinen ajo tehdään minuutin kuluttua käynnistyksestä (jotta
//! käynnistys ei hidastu ja mahdollinen konfiguraatiovirhe näkyy heti
//! lokissa), sen jälkeen `SYNC_INTERVAL_HOURS` välein.

use std::time::Duration;

use tokio::task::JoinHandle;

use super::{Trigger, sync_all};
use crate::state::AppState;

const INITIAL_DELAY: Duration = Duration::from_secs(60);

/// Käynnistää ajastimen. Palauttaa `None`, jos ajastus on pois päältä
/// (`SYNC_INTERVAL_HOURS=0`) tai Polar-tunnuksia ei ole.
pub fn spawn(state: AppState) -> Option<JoinHandle<()>> {
    let hours = state.config.sync_interval_hours;
    if hours == 0 {
        tracing::info!("scheduled sync disabled (SYNC_INTERVAL_HOURS=0)");
        return None;
    }
    if state.polar.is_none() {
        tracing::info!("scheduled sync disabled (Polar not configured)");
        return None;
    }

    let period = Duration::from_secs(hours * 3600);
    Some(tokio::spawn(async move {
        tokio::time::sleep(INITIAL_DELAY).await;
        let mut interval = tokio::time::interval(period);
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        loop {
            interval.tick().await;
            tracing::info!(every_hours = hours, "scheduled sync tick");
            sync_all(&state, Trigger::Scheduled).await;
        }
    }))
}
