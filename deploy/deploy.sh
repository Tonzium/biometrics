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
