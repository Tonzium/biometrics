-- Demo-data kehityskantaan: luo omistajalle Polar-tilin (ilman oikeaa tokenia) ja 60 päivää
-- satunnaista dataa, jotta käyttöliittymän voi katselmoida ilman Polar-tunnuksia.
-- Ajo:  docker exec -i polar-data-hub-dev-db-1 psql -U polar -d polar < backend/scripts/demo_seed.sql
-- Vaatii, että backend on kerran käynnistetty (omistajakäyttäjä on luotu).
-- HUOM: älä aja tuotantokantaan. Oikea synkronointi ei korvaa DEMO-rivejä, poista ne:
--   DELETE FROM polar_accounts WHERE polar_user_id = 424242;
DO $$
DECLARE
  uid uuid;
  acc uuid;
  d date;
  i int;
  sports text[] := ARRAY['RUNNING','CYCLING','STRENGTH_TRAINING','WALKING','TRAIL_RUNNING','SWIMMING'];
  sp text;
BEGIN
  SELECT id INTO uid FROM app_users ORDER BY created_at LIMIT 1;
  INSERT INTO polar_accounts (app_user_id, polar_user_id, access_token_enc, member_id, last_sync_at)
  VALUES (uid, 424242, '\x00'::bytea, uid::text, now())
  ON CONFLICT (app_user_id) DO UPDATE SET last_sync_at = now()
  RETURNING id INTO acc;

  FOR i IN 0..59 LOOP
    d := current_date - i;
    INSERT INTO sleep_nights (polar_account_id, date, sleep_start_time, sleep_end_time, continuity, continuity_class,
      light_sleep_s, deep_sleep_s, rem_sleep_s, unrecognized_sleep_s, sleep_score, sleep_charge, sleep_goal_s, sleep_rating,
      total_interruption_s, short_interruption_s, long_interruption_s, sleep_cycles, raw)
    VALUES (acc, d, ((d - 1)::timestamp + time '23:10' - ((random()*40)::int * interval '1 minute')) AT TIME ZONE 'Europe/Helsinki',
      (d::timestamp + time '06:40' + ((random()*50)::int * interval '1 minute')) AT TIME ZONE 'Europe/Helsinki',
      2.5 + random()*2.4, 3 + (random()*2)::int,
      11000 + (random()*4000)::int, 4500 + (random()*2500)::int, 5000 + (random()*2500)::int, 200 + (random()*400)::int,
      62 + (random()*30)::int, 2 + (random()*3)::int, 28800, 3 + (random()*2)::int,
      600 + (random()*1800)::int, 400 + (random()*800)::int, 100 + (random()*900)::int, 4 + (random()*2)::int, '{}'::jsonb)
    ON CONFLICT DO NOTHING;

    INSERT INTO nightly_recharge (polar_account_id, date, heart_rate_avg, beat_to_beat_avg_ms, hrv_avg_ms, breathing_rate_avg,
      nightly_recharge_status, ans_charge, ans_charge_status, raw)
    VALUES (acc, d, 48 + (random()*10)::int, 1050 + (random()*150)::int, 35 + (random()*30)::int, 12.5 + random()*3,
      2 + (random()*4)::int, -2 + random()*5, 2 + (random()*3)::int, '{}'::jsonb)
    ON CONFLICT DO NOTHING;

    INSERT INTO daily_activity (polar_account_id, date, start_time, end_time, active_duration_s, inactive_duration_s,
      daily_activity_pct, calories, active_calories, steps, inactivity_alert_count, distance_from_steps_m, raw)
    VALUES (acc, d, d::timestamp, d::timestamp + interval '23:59:59', 5400 + (random()*9000)::int, 40000 + (random()*20000)::int,
      50 + random()*70, 2200 + (random()*900)::int, 400 + (random()*900)::int, 4000 + (random()*11000)::int,
      (random()*3)::int, 3000 + (random()*8000)::int, '{}'::jsonb)
    ON CONFLICT DO NOTHING;

    INSERT INTO cardio_load (polar_account_id, date, status, cardio_load, strain, tolerance, cardio_load_ratio, raw)
    VALUES (acc, d, (ARRAY['MAINTAINING','PRODUCTIVE','PRODUCTIVE','OVERREACHING','DETRAINING'])[1 + (random()*4)::int],
      random()*180, 70 + random()*60, 80 + random()*40, 0.7 + random()*0.7, '{}'::jsonb)
    ON CONFLICT DO NOTHING;

    -- harjoitus noin joka toinen päivä
    IF random() < 0.55 THEN
      sp := sports[1 + (random()*5)::int];
      INSERT INTO exercises (id, polar_account_id, start_time, start_time_local, utc_offset_min, upload_time, duration_s, sport,
        detailed_sport_info, device, distance_m, calories, hr_avg, hr_max, training_load, has_route, running_index,
        fat_percentage, carbohydrate_percentage, protein_percentage, heart_rate_zones, raw)
      VALUES ('DEMO' || i, acc, (d::timestamp + time '17:15') AT TIME ZONE 'Europe/Helsinki', d::timestamp + time '17:15', 180,
        (d::timestamp + time '19:00') AT TIME ZONE 'Europe/Helsinki', 1500 + (random()*5400)::int, sp, sp, 'Polar Vantage V3',
        CASE WHEN sp IN ('STRENGTH_TRAINING') THEN NULL ELSE 3000 + random()*15000 END,
        250 + (random()*700)::int, 120 + (random()*40)::int, 160 + (random()*30)::int, 40 + random()*140,
        sp NOT IN ('STRENGTH_TRAINING','SWIMMING'), CASE WHEN sp LIKE '%RUNNING' THEN 45 + (random()*15)::int ELSE NULL END,
        20 + (random()*40)::int, 40 + (random()*40)::int, 2 + (random()*5)::int,
        '[{"index":1,"lower-limit":100,"upper-limit":120,"in-zone":"PT4M"},{"index":2,"lower-limit":120,"upper-limit":140,"in-zone":"PT12M"},{"index":3,"lower-limit":140,"upper-limit":160,"in-zone":"PT25M"},{"index":4,"lower-limit":160,"upper-limit":175,"in-zone":"PT14M"},{"index":5,"lower-limit":175,"upper-limit":190,"in-zone":"PT3M"}]'::jsonb,
        '{}'::jsonb)
      ON CONFLICT DO NOTHING;
    END IF;
  END LOOP;

  INSERT INTO physical_info (polar_account_id, date, modified_at, weight_kg, height_cm, maximum_heart_rate, resting_heart_rate,
    aerobic_threshold, anaerobic_threshold, vo2_max, weight_source, sleep_goal_s, raw)
  VALUES (acc, current_date - 3, now() - interval '3 days', 81.2, 182, 189, 47, 141, 169, 52, 'SOURCE_USER', 28800, '{}'::jsonb)
  ON CONFLICT DO NOTHING;
END $$;
SELECT (SELECT count(*) FROM exercises) AS exercises, (SELECT count(*) FROM sleep_nights) AS nights, (SELECT count(*) FROM daily_activity) AS days;
