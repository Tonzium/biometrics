//! Polar-datan upsertit. Jokainen funktio kirjoittaa yhden rivin ja
//! korvaa aiemman saman avaimen rivin (`ON CONFLICT ... DO UPDATE`), joten
//! synkronoinnin voi ajaa milloin tahansa uudelleen ilman duplikaatteja.

use anyhow::{Context, anyhow};
use polar_client::{
    duration::parse_iso8601_seconds,
    models::{
        CardioLoad, DailyActivity, Exercise, Fetched, NightlyRecharge, PhysicalInfo, SleepNight,
    },
};
use sqlx::PgPool;
use uuid::Uuid;

fn small(v: Option<i32>) -> Option<i16> {
    v.and_then(|x| i16::try_from(x).ok())
}

fn real(v: Option<f64>) -> Option<f32> {
    v.map(|x| x as f32)
}

fn seconds(iso: Option<&str>) -> Option<i32> {
    iso.and_then(parse_iso8601_seconds)
        .and_then(|s| i32::try_from(s).ok())
}

pub async fn upsert_exercise(
    pool: &PgPool,
    account_id: Uuid,
    item: &Fetched<Exercise>,
) -> anyhow::Result<()> {
    let e = &item.data;
    let duration_s = seconds(Some(&e.duration))
        .ok_or_else(|| anyhow!("exercise {} has invalid duration {:?}", e.id, e.duration))?;
    let hr = e.heart_rate.as_ref();

    sqlx::query!(
        r#"
        INSERT INTO exercises (
            id, polar_account_id, start_time, start_time_local, utc_offset_min, upload_time,
            duration_s, sport, detailed_sport_info, device, device_id, distance_m, calories,
            hr_avg, hr_max, training_load, has_route, running_index, fat_percentage,
            carbohydrate_percentage, protein_percentage, heart_rate_zones, training_load_pro, raw
        )
        SELECT
            $1::text, $2::uuid, $3::timestamptz, $4::timestamp, $5::integer, $6::timestamptz,
            $7::integer, $8::text, $9::text, $10::text, $11::text, $12::real, $13::integer,
            $14::smallint, $15::smallint, $16::real, $17::boolean, $18::smallint, $19::smallint,
            $20::smallint, $21::smallint, $22::jsonb, $23::jsonb, $24::jsonb
            -- Omistajan poistamaa harjoitusta ei tuoda takaisin (ks. migraatio 0007).
        WHERE NOT EXISTS (SELECT 1 FROM deleted_exercises d WHERE d.id = $1)
        ON CONFLICT (id) DO UPDATE SET
            polar_account_id = EXCLUDED.polar_account_id,
            start_time = EXCLUDED.start_time, start_time_local = EXCLUDED.start_time_local,
            utc_offset_min = EXCLUDED.utc_offset_min, upload_time = EXCLUDED.upload_time,
            duration_s = EXCLUDED.duration_s, sport = EXCLUDED.sport,
            detailed_sport_info = EXCLUDED.detailed_sport_info, device = EXCLUDED.device,
            device_id = EXCLUDED.device_id, distance_m = EXCLUDED.distance_m,
            calories = EXCLUDED.calories, hr_avg = EXCLUDED.hr_avg, hr_max = EXCLUDED.hr_max,
            training_load = EXCLUDED.training_load, has_route = EXCLUDED.has_route,
            running_index = EXCLUDED.running_index, fat_percentage = EXCLUDED.fat_percentage,
            carbohydrate_percentage = EXCLUDED.carbohydrate_percentage,
            protein_percentage = EXCLUDED.protein_percentage,
            heart_rate_zones = EXCLUDED.heart_rate_zones,
            training_load_pro = EXCLUDED.training_load_pro,
            raw = EXCLUDED.raw, synced_at = now()
        "#,
        e.id,
        account_id,
        e.start_time_utc(),
        e.start_time,
        e.start_time_utc_offset.unwrap_or(0),
        e.upload_time,
        duration_s,
        e.sport,
        e.detailed_sport_info,
        e.device,
        e.device_id,
        real(e.distance),
        e.calories,
        small(hr.and_then(|h| h.average)),
        small(hr.and_then(|h| h.maximum)),
        real(e.training_load),
        e.has_route,
        small(e.running_index),
        small(e.fat_percentage),
        small(e.carbohydrate_percentage),
        small(e.protein_percentage),
        e.heart_rate_zones,
        e.training_load_pro,
        item.raw
    )
    .execute(pool)
    .await
    .context("upsert exercise")?;
    Ok(())
}

/// Omistajan poisto: kirjaa harjoituksen poistolistalle ja poistaa rivin
/// samassa transaktiossa, jotta seuraava synkronointi ei tuo sitä takaisin.
/// Palauttaa `false`, jos harjoitusta ei ollut tällä tilillä.
pub async fn delete_exercise(pool: &PgPool, account_id: Uuid, id: &str) -> sqlx::Result<bool> {
    let mut tx = pool.begin().await?;
    let deleted = sqlx::query!(
        "DELETE FROM exercises WHERE polar_account_id = $1 AND id = $2",
        account_id,
        id
    )
    .execute(&mut *tx)
    .await?
    .rows_affected();
    if deleted == 0 {
        tx.rollback().await?;
        return Ok(false);
    }
    sqlx::query!(
        "INSERT INTO deleted_exercises (id, polar_account_id) VALUES ($1, $2)
         ON CONFLICT (id) DO UPDATE SET deleted_at = now()",
        id,
        account_id
    )
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(true)
}

pub async fn upsert_sleep(
    pool: &PgPool,
    account_id: Uuid,
    item: &Fetched<SleepNight>,
) -> anyhow::Result<()> {
    let n = &item.data;
    sqlx::query!(
        r#"
        INSERT INTO sleep_nights (
            polar_account_id, date, sleep_start_time, sleep_end_time, device_id, continuity,
            continuity_class, light_sleep_s, deep_sleep_s, rem_sleep_s, unrecognized_sleep_s,
            sleep_score, sleep_charge, sleep_goal_s, sleep_rating, total_interruption_s,
            short_interruption_s, long_interruption_s, sleep_cycles, group_duration_score,
            group_solidity_score, group_regeneration_score, hypnogram, heart_rate_samples, raw
        ) VALUES (
            $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19,
            $20, $21, $22, $23, $24, $25
        )
        ON CONFLICT (polar_account_id, date) DO UPDATE SET
            sleep_start_time = EXCLUDED.sleep_start_time, sleep_end_time = EXCLUDED.sleep_end_time,
            device_id = EXCLUDED.device_id, continuity = EXCLUDED.continuity,
            continuity_class = EXCLUDED.continuity_class, light_sleep_s = EXCLUDED.light_sleep_s,
            deep_sleep_s = EXCLUDED.deep_sleep_s, rem_sleep_s = EXCLUDED.rem_sleep_s,
            unrecognized_sleep_s = EXCLUDED.unrecognized_sleep_s, sleep_score = EXCLUDED.sleep_score,
            sleep_charge = EXCLUDED.sleep_charge, sleep_goal_s = EXCLUDED.sleep_goal_s,
            sleep_rating = EXCLUDED.sleep_rating, total_interruption_s = EXCLUDED.total_interruption_s,
            short_interruption_s = EXCLUDED.short_interruption_s,
            long_interruption_s = EXCLUDED.long_interruption_s, sleep_cycles = EXCLUDED.sleep_cycles,
            group_duration_score = EXCLUDED.group_duration_score,
            group_solidity_score = EXCLUDED.group_solidity_score,
            group_regeneration_score = EXCLUDED.group_regeneration_score,
            hypnogram = EXCLUDED.hypnogram, heart_rate_samples = EXCLUDED.heart_rate_samples,
            raw = EXCLUDED.raw, synced_at = now()
        "#,
        account_id,
        n.date,
        n.sleep_start_time,
        n.sleep_end_time,
        n.device_id,
        real(n.continuity),
        small(n.continuity_class),
        n.light_sleep,
        n.deep_sleep,
        n.rem_sleep,
        n.unrecognized_sleep_stage,
        small(n.sleep_score),
        small(n.sleep_charge),
        n.sleep_goal,
        small(n.sleep_rating),
        n.total_interruption_duration,
        n.short_interruption_duration,
        n.long_interruption_duration,
        small(n.sleep_cycles),
        real(n.group_duration_score),
        real(n.group_solidity_score),
        real(n.group_regeneration_score),
        n.hypnogram,
        n.heart_rate_samples,
        item.raw
    )
    .execute(pool)
    .await
    .context("upsert sleep night")?;
    Ok(())
}

pub async fn upsert_recharge(
    pool: &PgPool,
    account_id: Uuid,
    item: &Fetched<NightlyRecharge>,
) -> anyhow::Result<()> {
    let r = &item.data;
    sqlx::query!(
        r#"
        INSERT INTO nightly_recharge (
            polar_account_id, date, heart_rate_avg, beat_to_beat_avg_ms, hrv_avg_ms,
            breathing_rate_avg, nightly_recharge_status, ans_charge, ans_charge_status,
            hrv_samples, breathing_samples, raw
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
        ON CONFLICT (polar_account_id, date) DO UPDATE SET
            heart_rate_avg = EXCLUDED.heart_rate_avg, beat_to_beat_avg_ms = EXCLUDED.beat_to_beat_avg_ms,
            hrv_avg_ms = EXCLUDED.hrv_avg_ms, breathing_rate_avg = EXCLUDED.breathing_rate_avg,
            nightly_recharge_status = EXCLUDED.nightly_recharge_status, ans_charge = EXCLUDED.ans_charge,
            ans_charge_status = EXCLUDED.ans_charge_status, hrv_samples = EXCLUDED.hrv_samples,
            breathing_samples = EXCLUDED.breathing_samples, raw = EXCLUDED.raw, synced_at = now()
        "#,
        account_id,
        r.date,
        small(r.heart_rate_avg),
        small(r.beat_to_beat_avg),
        small(r.heart_rate_variability_avg),
        real(r.breathing_rate_avg),
        small(r.nightly_recharge_status),
        real(r.ans_charge),
        small(r.ans_charge_status),
        r.hrv_samples,
        r.breathing_samples,
        item.raw
    )
    .execute(pool)
    .await
    .context("upsert nightly recharge")?;
    Ok(())
}

pub async fn upsert_activity(
    pool: &PgPool,
    account_id: Uuid,
    item: &Fetched<DailyActivity>,
) -> anyhow::Result<()> {
    let a = &item.data;
    sqlx::query!(
        r#"
        INSERT INTO daily_activity (
            polar_account_id, date, start_time, end_time, active_duration_s, inactive_duration_s,
            daily_activity_pct, calories, active_calories, steps, inactivity_alert_count,
            distance_from_steps_m, samples, raw
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)
        ON CONFLICT (polar_account_id, date) DO UPDATE SET
            start_time = EXCLUDED.start_time, end_time = EXCLUDED.end_time,
            active_duration_s = EXCLUDED.active_duration_s,
            inactive_duration_s = EXCLUDED.inactive_duration_s,
            daily_activity_pct = EXCLUDED.daily_activity_pct, calories = EXCLUDED.calories,
            active_calories = EXCLUDED.active_calories, steps = EXCLUDED.steps,
            inactivity_alert_count = EXCLUDED.inactivity_alert_count,
            distance_from_steps_m = EXCLUDED.distance_from_steps_m, samples = EXCLUDED.samples,
            raw = EXCLUDED.raw, synced_at = now()
        "#,
        account_id,
        a.date(),
        Some(a.start_time),
        a.end_time,
        seconds(a.active_duration.as_deref()),
        seconds(a.inactive_duration.as_deref()),
        real(a.daily_activity),
        a.calories,
        a.active_calories,
        a.steps,
        small(a.inactivity_alert_count),
        real(a.distance_from_steps),
        a.samples,
        item.raw
    )
    .execute(pool)
    .await
    .context("upsert daily activity")?;
    Ok(())
}

pub async fn upsert_physical_info(
    pool: &PgPool,
    account_id: Uuid,
    item: &Fetched<PhysicalInfo>,
) -> anyhow::Result<()> {
    let p = &item.data;
    sqlx::query!(
        r#"
        INSERT INTO physical_info (
            polar_account_id, date, modified_at, weight_kg, height_cm, maximum_heart_rate,
            resting_heart_rate, aerobic_threshold, anaerobic_threshold, vo2_max, weight_source,
            training_background, typical_day, sleep_goal_s, raw
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)
        ON CONFLICT (polar_account_id, date) DO UPDATE SET
            modified_at = EXCLUDED.modified_at, weight_kg = EXCLUDED.weight_kg,
            height_cm = EXCLUDED.height_cm, maximum_heart_rate = EXCLUDED.maximum_heart_rate,
            resting_heart_rate = EXCLUDED.resting_heart_rate,
            aerobic_threshold = EXCLUDED.aerobic_threshold,
            anaerobic_threshold = EXCLUDED.anaerobic_threshold, vo2_max = EXCLUDED.vo2_max,
            weight_source = EXCLUDED.weight_source, training_background = EXCLUDED.training_background,
            typical_day = EXCLUDED.typical_day, sleep_goal_s = EXCLUDED.sleep_goal_s,
            raw = EXCLUDED.raw, synced_at = now()
        "#,
        account_id,
        p.modified.date_naive(),
        p.modified,
        real(p.weight),
        real(p.height),
        small(p.maximum_heart_rate),
        small(p.resting_heart_rate),
        small(p.aerobic_threshold),
        small(p.anaerobic_threshold),
        small(p.vo2_max),
        p.weight_source,
        p.training_background,
        p.typical_day,
        seconds(p.sleep_goal.as_deref()),
        item.raw
    )
    .execute(pool)
    .await
    .context("upsert physical info")?;
    Ok(())
}

pub async fn upsert_cardio_load(
    pool: &PgPool,
    account_id: Uuid,
    item: &Fetched<CardioLoad>,
) -> anyhow::Result<()> {
    let c = &item.data;
    sqlx::query!(
        r#"
        INSERT INTO cardio_load (
            polar_account_id, date, status, cardio_load, strain, tolerance, cardio_load_ratio,
            cardio_load_level, raw
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
        ON CONFLICT (polar_account_id, date) DO UPDATE SET
            status = EXCLUDED.status, cardio_load = EXCLUDED.cardio_load, strain = EXCLUDED.strain,
            tolerance = EXCLUDED.tolerance, cardio_load_ratio = EXCLUDED.cardio_load_ratio,
            cardio_load_level = EXCLUDED.cardio_load_level, raw = EXCLUDED.raw, synced_at = now()
        "#,
        account_id,
        c.date,
        c.cardio_load_status,
        real(c.cardio_load),
        real(c.strain),
        real(c.tolerance),
        real(c.cardio_load_ratio),
        c.cardio_load_level,
        item.raw
    )
    .execute(pool)
    .await
    .context("upsert cardio load")?;
    Ok(())
}
