#!/usr/bin/env bash
# Ottaa tietokannasta pg_dump-varmuuskopion, säilyttää paikallisesti viimeiset 30
# päivää ja lähettää salatun kopion koneen ulkopuolelle.
#
# Ajetaan repon juuressa (tai cronista absoluuttisella polulla):
#   ./deploy/backup.sh [kohdekansio]
#
# PALAUTUS paikallisesta kopiosta:
#   gunzip -c backups/polar-2026-09-12.sql.gz \
#     | docker compose -f deploy/docker-compose.yml exec -T db psql -U polar -d polar
#
# PALAUTUS etäkopiosta (vaatii yksityisen age-avaimen, joka EI ole palvelimella):
#   rclone copyto "$BACKUP_REMOTE/polar-2026-09-12.sql.gz.age" ./palautus.age
#   age -d -i backup.key palautus.age | gunzip \
#     | docker compose -f deploy/docker-compose.yml exec -T db psql -U polar -d polar
#
# Paikallinen kopio on salaamaton: se on samalla koneella kuin kanta itse, joten
# salaus ei suojaisi miltään uudelta ja se hankaloittaisi rutiinipalautusta.
# Vain omistaja saa lukea tiedostot (umask 077 + chmod 600). Koneelta POIS
# lähtevä kopio salataan aina, koska se menee kolmannen osapuolen haltuun.
set -euo pipefail
umask 077

cd "$(dirname "$0")/.."
COMPOSE="docker compose -f deploy/docker-compose.yml"
DIR="${1:-backups}"
mkdir -p "$DIR"

set -a
# shellcheck source=/dev/null
source deploy/.env
set +a
USER_="${POSTGRES_USER:-polar}"
DB_="${POSTGRES_DB:-polar}"

# pg_dump ajetaan superuserina: se lukee kaiken riippumatta siitä, kuuluvatko
# taulut sovellusroolille (ks. deploy/sql/app-role-handover.sql).
FILE="$DIR/polar-$(date +%F).sql.gz"
$COMPOSE exec -T db pg_dump -U "$USER_" "$DB_" | gzip > "$FILE"
chmod 600 "$FILE"

# Järjen tarkistus ennen kuin mitään lähetetään ulos: tyhjä tai typistynyt dump
# pakkautuu ja salautuu aivan yhtä hyvin kuin kunnollinen, joten ilman tätä
# rikkinäinen kopio voisi korvata toimivat etäkopiot päivä kerrallaan.
# Mitattu tästä kannasta: pelkkä skeema (uusi asennus, ei dataa) pakkautuu
# 3567 tavuun ja dump demodatalla 15193 tavuun, kun taas epäonnistunut dump on
# 20 tavua ja pelkkä virheilmoitus 54. Raja 1000 on siis selvästi molempien
# välissä.
MIN_BYTES=1000
SIZE=$(stat -c %s "$FILE")
if [[ "$SIZE" -lt "$MIN_BYTES" ]]; then
    echo "VIRHE: $FILE on vain $SIZE tavua (alle $MIN_BYTES). Dump epäonnistui, ei lähetetä etäkopiota." >&2
    exit 1
fi
echo "kirjoitettu $FILE ($(du -h "$FILE" | cut -f1))"

# Poista yli 30 päivää vanhat paikalliset kopiot
find "$DIR" -name 'polar-*.sql.gz' -mtime +30 -delete

# --- Etäkopio ---------------------------------------------------------------
# Vaatii kaksi asetusta deploy/.env:issä: BACKUP_AGE_RECIPIENT (julkinen
# age-avain) ja BACKUP_REMOTE (rclone-kohde). Kohde voi olla mikä tahansa
# rclonen tukema paikka; tunnukset annetaan RCLONE_CONFIG_*-muuttujina, jotka
# tulevat .env:istä automaattisesti (set -a), joten erillistä rclone.conf-
# tiedostoa ei tarvita. Ks. deploy/.env.example ja docs/JULKAISU.md.
if [[ -z "${BACKUP_AGE_RECIPIENT:-}" || -z "${BACKUP_REMOTE:-}" ]]; then
    echo "HUOM: etäkopiota ei ole määritetty (BACKUP_AGE_RECIPIENT / BACKUP_REMOTE puuttuu)." >&2
    echo "      Kopio on vain tällä koneella: levyrikko tai kiristyshaittaohjelma vie sen mukanaan." >&2
    exit 0
fi

for tool in age rclone; do
    if ! command -v "$tool" >/dev/null 2>&1; then
        echo "VIRHE: $tool puuttuu. Asenna: sudo apt-get install -y age rclone" >&2
        exit 1
    fi
done

# Jos rclonen omaa asetustiedostoa ei ole, vaiennetaan sen "Config file not
# found" -ilmoitus. Ympäristömuuttujilla määritelty kohde toimii ilman sitä.
if [[ ! -f "${HOME}/.config/rclone/rclone.conf" ]]; then
    export RCLONE_CONFIG=/dev/null
fi

# Cron-ajolle tärkeää: kuollut päätepiste ei saa jumittaa ajoa minuuteiksi.
# Ilman näitä rclone yrittää oletuksena kolmesti pitkillä aikakatkaisuilla.
RCLONE_OPTS=(--retries 2 --low-level-retries 2 --contimeout 20s --timeout 60s)

ENC="$FILE.age"
# Salattu välitiedosto siivotaan aina, myös virheen sattuessa.
trap 'rm -f "$ENC"' EXIT
# Salataan julkisella avaimella: palvelimella ei ole yksityistä avainta, joten se
# ei voi purkaa omia vanhoja varmuuskopioitaan. Tämä on tarkoitus.
age -r "$BACKUP_AGE_RECIPIENT" -o "$ENC" "$FILE"

REMOTE_NAME=$(basename "$ENC")
rclone "${RCLONE_OPTS[@]}" copyto "$ENC" "$BACKUP_REMOTE/$REMOTE_NAME"

# Varmistetaan että kopio todella on perillä ja oikean kokoinen.
LOCAL_BYTES=$(stat -c %s "$ENC")
REMOTE_BYTES=$(rclone "${RCLONE_OPTS[@]}" lsl "$BACKUP_REMOTE" 2>/dev/null | awk -v f="$REMOTE_NAME" '$NF == f {print $1; exit}')
if [[ "$REMOTE_BYTES" != "$LOCAL_BYTES" ]]; then
    echo "VIRHE: etäkopion koko ($REMOTE_BYTES) ei täsmää paikalliseen ($LOCAL_BYTES)." >&2
    exit 1
fi
echo "etäkopio lähetetty: $BACKUP_REMOTE/$REMOTE_NAME ($LOCAL_BYTES tavua, salattu)"

# Etäkopioiden säilytys. Oletus on pidempi kuin paikallinen, koska etäkopio on
# se, joka on jäljellä kun kone on mennyt.
KEEP_DAYS="${BACKUP_REMOTE_KEEP_DAYS:-90}"
rclone "${RCLONE_OPTS[@]}" delete --min-age "${KEEP_DAYS}d" "$BACKUP_REMOTE"
echo "etäkopioista poistettu yli $KEEP_DAYS päivää vanhat"
