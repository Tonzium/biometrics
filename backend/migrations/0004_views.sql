-- Raportointinäkymät käyttöliittymän kaavioita varten.
-- Korvaavat vanhan projektin dbt-mallit (fct_daily_wellness, agg_weekly_summary).

-- Päivätaso: uni + palautuminen + aktiivisuus + cardio load samalla rivillä.
-- FULL OUTER JOIN, koska kaikilta päiviltä ei ole kaikkea dataa.
CREATE VIEW v_daily_wellness AS
WITH days AS (
    SELECT polar_account_id, date FROM sleep_nights
    UNION
    SELECT polar_account_id, date FROM nightly_recharge
    UNION
    SELECT polar_account_id, date FROM daily_activity
    UNION
    SELECT polar_account_id, date FROM cardio_load
)
SELECT
    d.polar_account_id,
    d.date,
    -- uni
    s.sleep_score,
    s.sleep_charge,
    (s.light_sleep_s + s.deep_sleep_s + s.rem_sleep_s)      AS sleep_total_s,
    s.light_sleep_s,
    s.deep_sleep_s,
    s.rem_sleep_s,
    s.total_interruption_s,
    s.continuity,
    -- palautuminen
    r.nightly_recharge_status,
    r.ans_charge,
    r.ans_charge_status,
    r.heart_rate_avg                                         AS night_hr_avg,
    r.hrv_avg_ms                                             AS night_hrv_avg_ms,
    r.breathing_rate_avg                                     AS night_breathing_rate_avg,
    -- aktiivisuus
    a.steps,
    a.calories,
    a.active_calories,
    a.active_duration_s,
    a.daily_activity_pct,
    a.distance_from_steps_m,
    -- kuormitus
    c.status                                                 AS cardio_load_status,
    c.cardio_load,
    c.strain,
    c.tolerance,
    c.cardio_load_ratio
FROM days d
LEFT JOIN sleep_nights     s USING (polar_account_id, date)
LEFT JOIN nightly_recharge r USING (polar_account_id, date)
LEFT JOIN daily_activity   a USING (polar_account_id, date)
LEFT JOIN cardio_load      c USING (polar_account_id, date);

-- Viikkotaso (ISO-viikko, alkaa maanantaina): harjoitusmäärät + keskimääräinen uni/palautuminen.
CREATE VIEW v_weekly_summary AS
WITH ex AS (
    SELECT
        polar_account_id,
        date_trunc('week', start_time)::date          AS week_start,
        count(*)                                      AS exercise_count,
        sum(duration_s)                               AS total_duration_s,
        sum(distance_m)                               AS total_distance_m,
        sum(calories)                                 AS total_calories,
        sum(training_load)                            AS total_training_load,
        avg(hr_avg)                                   AS avg_hr,
        max(hr_max)                                   AS max_hr
    FROM exercises
    GROUP BY polar_account_id, date_trunc('week', start_time)::date
),
wl AS (
    SELECT
        polar_account_id,
        date_trunc('week', date)::date                AS week_start,
        avg(sleep_score)                              AS avg_sleep_score,
        avg(sleep_total_s)                            AS avg_sleep_total_s,
        avg(nightly_recharge_status)                  AS avg_recharge_status,
        avg(night_hrv_avg_ms)                         AS avg_night_hrv_ms,
        sum(steps)                                    AS total_steps,
        count(sleep_score)                            AS nights_with_sleep
    FROM v_daily_wellness
    GROUP BY polar_account_id, date_trunc('week', date)::date
)
SELECT
    coalesce(ex.polar_account_id, wl.polar_account_id) AS polar_account_id,
    coalesce(ex.week_start, wl.week_start)             AS week_start,
    coalesce(ex.exercise_count, 0)                     AS exercise_count,
    coalesce(ex.total_duration_s, 0)                   AS total_duration_s,
    ex.total_distance_m,
    ex.total_calories,
    ex.total_training_load,
    ex.avg_hr,
    ex.max_hr,
    wl.avg_sleep_score,
    wl.avg_sleep_total_s,
    wl.avg_recharge_status,
    wl.avg_night_hrv_ms,
    wl.total_steps,
    coalesce(wl.nights_with_sleep, 0)                  AS nights_with_sleep
FROM ex
FULL OUTER JOIN wl USING (polar_account_id, week_start);
