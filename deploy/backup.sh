#!/usr/bin/env bash
# Ottaa tietokannasta pg_dump-varmuuskopion ja säilyttää viimeiset 30.
# Ajetaan repon juuressa (tai cronista absoluuttisella polulla):
#   ./deploy/backup.sh [kohdekansio]
# Palautus:
#   gunzip -c backups/polar-2026-09-11.sql.gz | docker compose -f deploy/docker-compose.yml exec -T db psql -U polar -d polar
set -euo pipefail

cd "$(dirname "$0")/.."
COMPOSE="docker compose -f deploy/docker-compose.yml"
DIR="${1:-backups}"
mkdir -p "$DIR"

set -a; source deploy/.env; set +a
USER_="${POSTGRES_USER:-polar}"
DB_="${POSTGRES_DB:-polar}"

FILE="$DIR/polar-$(date +%F).sql.gz"
$COMPOSE exec -T db pg_dump -U "$USER_" "$DB_" | gzip > "$FILE"
echo "kirjoitettu $FILE ($(du -h "$FILE" | cut -f1))"

# Poista yli 30 päivää vanhat
find "$DIR" -name 'polar-*.sql.gz' -mtime +30 -delete
