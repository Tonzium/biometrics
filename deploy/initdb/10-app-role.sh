#!/bin/sh
# Luo sovellukselle oman, rajoitetun kantaroolin ENSIMMÄISELLÄ käynnistyksellä.
#
# postgres-image ajaa /docker-entrypoint-initdb.d/-skriptit vain silloin, kun
# datahakemisto on tyhjä. Olemassa olevaan kantaan rooli lisätään kertaluonteisesti
# käsin: deploy/sql/app-role-handover.sql (ks. docs/JULKAISU.md).
#
# Rooli ei ole superuser, joten se ei voi ajaa COPY ... FROM PROGRAM -komentoa
# (SQL:stä käyttöjärjestelmäkomentoihin), lukea palvelimen tiedostoja, luoda tai
# muuttaa rooleja eikä asentaa mitä tahansa laajennusta. Migraatiot se pystyy
# ajamaan, koska se omistaa public-skeeman ja pgcrypto on "trusted"-laajennus.
#
# Jos DB_APP_USER tai DB_APP_PASSWORD on tyhjä, roolia ei luoda ja kanta jää
# entiselleen. Silloin myös api käyttää compose-tiedoston oletuksen mukaan
# superuseria, eli mikään ei hajoa.
set -e

if [ -z "${DB_APP_USER:-}" ] || [ -z "${DB_APP_PASSWORD:-}" ]; then
    echo "10-app-role: DB_APP_USER tai DB_APP_PASSWORD puuttuu, sovellusroolia ei luoda."
    echo "10-app-role: api käyttää tällöin superuseria, kuten ennen."
else
    # Arvot välitetään psql-muuttujina, jotta lainausmerkit ja erikoismerkit
    # eivät riko komentoa. :'x' lainaa literaalina, :"x" tunnisteena.
    psql -v ON_ERROR_STOP=1 \
         -v role="$DB_APP_USER" -v pw="$DB_APP_PASSWORD" -v db="$POSTGRES_DB" \
         --username "$POSTGRES_USER" --dbname "$POSTGRES_DB" <<'SQL'
CREATE ROLE :"role" LOGIN PASSWORD :'pw'
    NOSUPERUSER NOCREATEDB NOCREATEROLE NOREPLICATION NOBYPASSRLS;

-- CREATE tarvitaan, jotta rooli voi luoda taulut ja asentaa pgcryptoa
-- (trusted-laajennus). TEMPORARY on sqlx:n väliaikaistauluja varten.
GRANT CONNECT, CREATE, TEMPORARY ON DATABASE :"db" TO :"role";

-- Skeeman omistajuus antaa oikeuden luoda ja muuttaa tauluja, eli ajaa
-- migraatiot. Kanta itse jää superuserin omistukseen.
ALTER SCHEMA public OWNER TO :"role";
SQL
    echo "10-app-role: sovellusrooli $DB_APP_USER luotu"
fi
