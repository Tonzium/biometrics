#!/usr/bin/env bash
# Päivittää sovelluksen palvelimella: hakee uusimman koodin (compose-tiedostot,
# skriptit), vetää CI:n rakentamat imaget ja käynnistää muuttuneet kontit.
# Ajetaan repon juuressa:  ./deploy/deploy.sh
# Rakenna itse lähdekoodista:  BUILD=1 ./deploy/deploy.sh
set -euo pipefail

cd "$(dirname "$0")/.."
COMPOSE="docker compose -f deploy/docker-compose.yml"

if [[ ! -f deploy/.env ]]; then
    echo "deploy/.env puuttuu. Kopioi deploy/.env.example ja täytä arvot." >&2
    exit 1
fi

# Sovellusroolin muuttujat on asetettava molemmat tai ei kumpaakaan. Vain
# toinen asetettuna DATABASE_URL saisi väärän yhdistelmän (esim. superuserin
# nimen ja sovellusroolin salasanan), ja api jäisi uudelleenkäynnistyssilmukkaan
# autentikointivirheen takia. Kiinni otetaan ennen kuin mitään käynnistetään.
app_user_set=$(grep -cE '^DB_APP_USER=.+' deploy/.env || true)
app_pw_set=$(grep -cE '^DB_APP_PASSWORD=.+' deploy/.env || true)
if [[ "$app_user_set" != "$app_pw_set" ]]; then
    echo "deploy/.env: aseta sekä DB_APP_USER että DB_APP_PASSWORD, tai kumpaakaan ei lainkaan." >&2
    echo "Ks. docs/JULKAISU.md (sovelluksen kantarooli)." >&2
    exit 1
fi

echo "== git pull"
git pull --ff-only

if [[ "${BUILD:-0}" == "1" ]]; then
    echo "== build lähdekoodista & up"
    $COMPOSE -f deploy/docker-compose.build.yml up -d --build --remove-orphans
else
    echo "== pull & up"
    $COMPOSE pull --quiet
    $COMPOSE up -d --remove-orphans
fi

echo "== odotetaan api:n healthcheckiä"
for _ in $(seq 1 30); do
    if $COMPOSE ps --format '{{.Name}} {{.Status}}' | grep -q 'api-1 Up.*healthy'; then
        break
    fi
    sleep 3
done
$COMPOSE ps

echo "== siivotaan vanhat imaget"
docker image prune -f >/dev/null

echo "== valmis. Lokit: $COMPOSE logs -f api"
