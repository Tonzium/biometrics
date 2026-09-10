-- Polar AccessLink -datasta johdetut taulut.
--
-- Periaatteet:
--   * Jokaisella rivillä on `raw jsonb` = Polarin alkuperäinen vastaus, jotta
--     myöhemmin voidaan purkaa kenttiä, joita ei vielä ole sarakkeina.
--   * Luonnollinen avain (Polarin id tai tili+päivä) on pääavain, jolloin
--     synkronointi on idempotentti upsert (`ON CONFLICT ... DO UPDATE`).
--   * Kestot tallennetaan sekunteina (Polar antaa ISO 8601 -kestoja).
--   * `synced_at` kertoo, milloin rivi viimeksi haettiin Polarista.

-- ---------------------------------------------------------------------------
-- Harjoitukset  (GET /v3/exercises, GET /v3/exercises/{id})
-- ---------------------------------------------------------------------------
CREATE TABLE exercises (
    id                      text        PRIMARY KEY,           -- Polarin hash-id
    polar_account_id        uuid        NOT NULL REFERENCES polar_accounts(id) ON DELETE CASCADE,
    start_time              timestamptz NOT NULL,              -- UTC (paikallinen aika - offset)
    start_time_local        timestamp   NOT NULL,              -- Polarin antama paikallinen aika
    utc_offset_min          integer     NOT NULL DEFAULT 0,
    upload_time             timestamptz,
    duration_s              integer     NOT NULL CHECK (duration_s >= 0),
    sport                   text        NOT NULL,
    detailed_sport_info     text,
    device                  text,
    device_id               text,
    distance_m              real,
    calories                integer,
    hr_avg                  smallint,
    hr_max                  smallint,
    training_load           real,
    has_route               boolean     NOT NULL DEFAULT false,
    running_index           smallint,
    fat_percentage          smallint,
    carbohydrate_percentage smallint,
    protein_percentage      smallint,
    heart_rate_zones        jsonb,
    training_load_pro       jsonb,
    raw                     jsonb       NOT NULL,
    synced_at               timestamptz NOT NULL DEFAULT now(),
    created_at              timestamptz NOT NULL DEFAULT now(),
    updated_at              timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX exercises_account_start_idx ON exercises (polar_account_id, start_time DESC);
CREATE INDEX exercises_account_sport_idx ON exercises (polar_account_id, sport);

CREATE TRIGGER exercises_set_updated_at
    BEFORE UPDATE ON exercises
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();

-- ---------------------------------------------------------------------------
-- Uni  (GET /v3/users/sleep  -> { "nights": [...] })
-- ---------------------------------------------------------------------------
CREATE TABLE sleep_nights (
    polar_account_id         uuid        NOT NULL REFERENCES polar_accounts(id) ON DELETE CASCADE,
    date                     date        NOT NULL,             -- herääminen tälle päivälle
    sleep_start_time         timestamptz NOT NULL,
    sleep_end_time           timestamptz NOT NULL,
    device_id                text,
    continuity               real,                             -- 1.0-5.0
    continuity_class         smallint,                         -- 1-5
    light_sleep_s            integer,
    deep_sleep_s             integer,
    rem_sleep_s              integer,
    unrecognized_sleep_s     integer,
    sleep_score              smallint,                         -- 1-100
    sleep_charge             smallint,                         -- 1-5 vs. 28 pv keskiarvo
    sleep_goal_s             integer,
    sleep_rating             smallint,                         -- käyttäjän oma arvio 1-5
    total_interruption_s     integer,
    short_interruption_s     integer,
    long_interruption_s      integer,
    sleep_cycles             smallint,
    group_duration_score     real,
    group_solidity_score     real,
    group_regeneration_score real,
    hypnogram                jsonb,                            -- {"00:39": 2, ...}
    heart_rate_samples       jsonb,                            -- {"00:41": 76, ...}
    raw                      jsonb       NOT NULL,
    synced_at                timestamptz NOT NULL DEFAULT now(),
    created_at               timestamptz NOT NULL DEFAULT now(),
    updated_at               timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (polar_account_id, date)
);

CREATE TRIGGER sleep_nights_set_updated_at
    BEFORE UPDATE ON sleep_nights
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();

-- ---------------------------------------------------------------------------
-- Nightly Recharge  (GET /v3/users/nightly-recharge -> { "recharges": [...] })
-- ---------------------------------------------------------------------------
CREATE TABLE nightly_recharge (
    polar_account_id        uuid        NOT NULL REFERENCES polar_accounts(id) ON DELETE CASCADE,
    date                    date        NOT NULL,
    heart_rate_avg          smallint,                          -- bpm
    beat_to_beat_avg_ms     smallint,
    hrv_avg_ms              smallint,                          -- heart_rate_variability_avg
    breathing_rate_avg      real,                              -- hengitystä/min
    nightly_recharge_status smallint,                          -- 1-6
    ans_charge              real,
    ans_charge_status       smallint,                          -- 1-5
    hrv_samples             jsonb,
    breathing_samples       jsonb,
    raw                     jsonb       NOT NULL,
    synced_at               timestamptz NOT NULL DEFAULT now(),
    created_at              timestamptz NOT NULL DEFAULT now(),
    updated_at              timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (polar_account_id, date)
);

CREATE TRIGGER nightly_recharge_set_updated_at
    BEFORE UPDATE ON nightly_recharge
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();

-- ---------------------------------------------------------------------------
-- Päiväaktiivisuus  (GET /v3/users/activities?from&to, GET /v3/users/activities/{date})
-- ---------------------------------------------------------------------------
CREATE TABLE daily_activity (
    polar_account_id       uuid        NOT NULL REFERENCES polar_accounts(id) ON DELETE CASCADE,
    date                   date        NOT NULL,
    start_time             timestamp,                          -- UTC ilman vyöhykettä, kuten Polar antaa
    end_time               timestamp,
    active_duration_s      integer,
    inactive_duration_s    integer,
    daily_activity_pct     real,                               -- päivätavoitteen täyttymis-%
    calories               integer,
    active_calories        integer,
    steps                  integer,
    inactivity_alert_count smallint,
    distance_from_steps_m  real,
    samples                jsonb,                              -- askel- ja vyöhykenäytteet, jos haettu
    raw                    jsonb       NOT NULL,
    synced_at              timestamptz NOT NULL DEFAULT now(),
    created_at             timestamptz NOT NULL DEFAULT now(),
    updated_at             timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (polar_account_id, date)
);

CREATE TRIGGER daily_activity_set_updated_at
    BEFORE UPDATE ON daily_activity
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();

-- ---------------------------------------------------------------------------
-- Fyysiset tiedot  (GET /v3/users/physical-info, palauttaa nykytilan)
-- Historia syntyy tallentamalla rivi `modified`-aikaleiman päivälle.
-- ---------------------------------------------------------------------------
CREATE TABLE physical_info (
    polar_account_id    uuid        NOT NULL REFERENCES polar_accounts(id) ON DELETE CASCADE,
    date                date        NOT NULL,                  -- modified::date
    modified_at         timestamptz NOT NULL,
    weight_kg           real,
    height_cm           real,
    maximum_heart_rate  smallint,
    resting_heart_rate  smallint,
    aerobic_threshold   smallint,
    anaerobic_threshold smallint,
    vo2_max             smallint,
    weight_source       text,
    training_background text,
    typical_day         text,
    sleep_goal_s        integer,
    raw                 jsonb       NOT NULL,
    synced_at           timestamptz NOT NULL DEFAULT now(),
    created_at          timestamptz NOT NULL DEFAULT now(),
    updated_at          timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (polar_account_id, date)
);

CREATE TRIGGER physical_info_set_updated_at
    BEFORE UPDATE ON physical_info
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();

-- ---------------------------------------------------------------------------
-- Cardio Load  (GET /v3/users/cardio-load)
-- ---------------------------------------------------------------------------
CREATE TABLE cardio_load (
    polar_account_id  uuid        NOT NULL REFERENCES polar_accounts(id) ON DELETE CASCADE,
    date              date        NOT NULL,
    status            text,                                    -- esim. PRODUCTIVE, OVERREACHING
    cardio_load       real,
    strain            real,
    tolerance         real,
    cardio_load_ratio real,
    cardio_load_level jsonb,                                   -- {"very_low":..,"low":..,...}
    raw               jsonb       NOT NULL,
    synced_at         timestamptz NOT NULL DEFAULT now(),
    created_at        timestamptz NOT NULL DEFAULT now(),
    updated_at        timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (polar_account_id, date)
);

CREATE TRIGGER cardio_load_set_updated_at
    BEFORE UPDATE ON cardio_load
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();

-- ---------------------------------------------------------------------------
-- Synkronointiajojen loki
-- ---------------------------------------------------------------------------
CREATE TABLE sync_runs (
    id               bigint      PRIMARY KEY GENERATED ALWAYS AS IDENTITY,
    polar_account_id uuid        NOT NULL REFERENCES polar_accounts(id) ON DELETE CASCADE,
    trigger          text        NOT NULL CHECK (trigger IN ('initial', 'manual', 'scheduled')),
    status           text        NOT NULL DEFAULT 'running'
                                 CHECK (status IN ('running', 'ok', 'partial', 'failed')),
    started_at       timestamptz NOT NULL DEFAULT now(),
    finished_at      timestamptz,
    counts           jsonb       NOT NULL DEFAULT '{}'::jsonb, -- {"exercises": 3, "sleep_nights": 7, ...}
    error            text
);

CREATE INDEX sync_runs_account_started_idx ON sync_runs (polar_account_id, started_at DESC);
