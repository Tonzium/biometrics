-- Viikkonäkymän keskiarvot palautetaan liukulukuina (avg() kokonaisluvuista
-- tuottaa Postgresissa NUMERIC-tyypin, jota rajapinta ei halua käsitellä).
-- Tyyppimuutos ei onnistu CREATE OR REPLACE VIEW -lauseella, joten näkymä
-- luodaan uudelleen.

DROP VIEW v_weekly_summary;

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
        avg(hr_avg)::double precision                 AS avg_hr,
        max(hr_max)                                   AS max_hr
    FROM exercises
    GROUP BY polar_account_id, date_trunc('week', start_time)::date
),
wl AS (
    SELECT
        polar_account_id,
        date_trunc('week', date)::date                AS week_start,
        avg(sleep_score)::double precision            AS avg_sleep_score,
        avg(sleep_total_s)::double precision          AS avg_sleep_total_s,
        avg(nightly_recharge_status)::double precision AS avg_recharge_status,
        avg(night_hrv_avg_ms)::double precision       AS avg_night_hrv_ms,
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
