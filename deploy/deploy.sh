#!/usr/bin/env bash
# Päivittää sovelluksen palvelimella: hakee uusimman koodin, rakentaa imaget ja
# käynnistää muuttuneet kontit. Ajetaan repon juuressa:  ./deploy/deploy.sh
set -euo pipefail

cd "$(dirname "$0")/.."
COMPOSE="docker compose -f deploy/docker-compose.yml"

if [[ ! -f deploy/.env ]]; then
    echo "deploy/.env puuttuu. Kopioi deploy/.env.example ja täytä arvot." >&2
    exit 1
fi

echo "== git pull"
git pull --ff-only

echo "== build & up"
$COMPOSE up -d --build --remove-orphans

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
