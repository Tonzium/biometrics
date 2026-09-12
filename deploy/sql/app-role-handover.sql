-- Kertaluonteinen siirto: olemassa oleva kanta superuserilta sovellusroolille.
--
-- Tarvitaan vain kannoille, jotka on jo alustettu. Uusi asennus saa roolin
-- suoraan deploy/initdb/10-app-role.sh:sta, koska postgres-image ajaa
-- init-skriptit vain tyhjään datahakemistoon.
--
-- Ajo (repon juuressa, kun pino on käynnissä):
--
--   set -a; . deploy/.env; set +a
--   docker compose -f deploy/docker-compose.yml exec -T db \
--     psql -U "$POSTGRES_USER" -d "$POSTGRES_DB" \
--          -v ON_ERROR_STOP=1 -v role="$DB_APP_USER" -v pw="$DB_APP_PASSWORD" \
--          -v db="$POSTGRES_DB" < deploy/sql/app-role-handover.sql
--
-- Koko siirto on yhdessä transaktiossa: jos jokin rivi epäonnistuu, mitään ei
-- muutu. Skripti ei ole idempotentti — toinen ajo kaatuu virheeseen "role
-- already exists", mikä on tarkoituksellista (sen jälkeen riittää salasanan
-- vaihto: ALTER ROLE ... PASSWORD).
--
-- HUOM 1: REASSIGN OWNED BY <superuser> ei toimi tässä. PostgreSQL hylkää sen
-- virheellä "cannot reassign ownership of objects owned by role ... because they
-- are required by the database system", koska bootstrap-superuser omistaa myös
-- järjestelmäobjekteja. Siksi omistajuus siirretään objekti kerrallaan.
--
-- HUOM 2: psql ei korvaa :muuttujia dollarilainausten ($$ ... $$) sisällä, joten
-- roolin nimi välitetään DO-lohkoon istuntomuuttujana (SET LOCAL + current_setting).

BEGIN;

-- Roolin nimi DO-lohkon käyttöön. SET LOCAL purkautuu COMMITissa, joten
-- lopun tarkistuskysely käyttää :'role'-muuttujaa suoraan.
SET LOCAL handover.role = :'role';

CREATE ROLE :"role" LOGIN PASSWORD :'pw'
    NOSUPERUSER NOCREATEDB NOCREATEROLE NOREPLICATION NOBYPASSRLS;

-- CREATE tarvitaan migraatioihin ja pgcryptoon (trusted-laajennus),
-- TEMPORARY sqlx:n väliaikaistauluihin.
GRANT CONNECT, CREATE, TEMPORARY ON DATABASE :"db" TO :"role";

-- Skeeman omistajuus antaa oikeuden luoda ja muuttaa tauluja. Kanta itse jää
-- superuserin omistukseen, eli sovellusrooli ei voi pudottaa koko kantaa.
ALTER SCHEMA public OWNER TO :"role";

-- Taulut, näkymät, sekvenssit ja omat funktiot sovellusroolille. Laajennuksen
-- (pgcrypto) omat funktiot jätetään koskematta: ne kuuluvat laajennukselle, ja
-- omistajan siirto rikkoisi sen. pg_depend.deptype = 'e' tunnistaa ne.
DO $handover$
DECLARE
    r record;
    app text := current_setting('handover.role');
BEGIN
    FOR r IN SELECT tablename FROM pg_tables WHERE schemaname = 'public' LOOP
        EXECUTE format('ALTER TABLE public.%I OWNER TO %I', r.tablename, app);
    END LOOP;

    FOR r IN SELECT viewname FROM pg_views WHERE schemaname = 'public' LOOP
        EXECUTE format('ALTER VIEW public.%I OWNER TO %I', r.viewname, app);
    END LOOP;

    FOR r IN SELECT sequencename FROM pg_sequences WHERE schemaname = 'public' LOOP
        EXECUTE format('ALTER SEQUENCE public.%I OWNER TO %I', r.sequencename, app);
    END LOOP;

    FOR r IN
        SELECT p.oid::regprocedure AS sig
        FROM pg_proc p
        JOIN pg_namespace n ON n.oid = p.pronamespace
        WHERE n.nspname = 'public'
          AND NOT EXISTS (
              SELECT 1 FROM pg_depend d WHERE d.objid = p.oid AND d.deptype = 'e'
          )
    LOOP
        EXECUTE format('ALTER FUNCTION %s OWNER TO %I', r.sig, app);
    END LOOP;
END
$handover$;

COMMIT;

-- Tarkistus: kaiken pitäisi nyt kuulua sovellusroolille ja kannan itsensä
-- edelleen superuserille.
SELECT 'skeema public' AS kohde, pg_get_userbyid(nspowner)::text AS omistaja
FROM pg_namespace WHERE nspname = 'public'
UNION ALL
SELECT 'tauluja sovellusroolilla', count(*)::text
FROM pg_tables WHERE schemaname = 'public' AND tableowner = :'role'
UNION ALL
SELECT 'tauluja yha superuserilla', count(*)::text
FROM pg_tables WHERE schemaname = 'public' AND tableowner = current_user
UNION ALL
SELECT 'kanta ' || current_database(), pg_get_userbyid(datdba)::text
FROM pg_database WHERE datname = current_database();
