# Polar Data Hub

Full stack -web-sovellus, joka hakee omat harjoitus-, uni-, palautumis- ja aktiivisuustiedot
Polar Flow -kellosta (Polar AccessLink API), tallentaa ne pysyvästi PostgreSQL-kantaan ja
näyttää ne React-käyttöliittymässä. KAMK:n Web-sovelluskehitys-kurssin lopputyö.

Suunnitelma ja arkkitehtuuri: [docs/SUUNNITELMA.md](docs/SUUNNITELMA.md)

## Pino

| Kerros | Teknologia |
|---|---|
| Backend | Rust 1.98 (edition 2024), axum 0.8, sqlx 0.9, reqwest 0.13 |
| Frontend | TypeScript 5.9, React 19, Vite 8, TanStack Query, react-router, Recharts |
| Tietokanta | PostgreSQL 18 |
| Julkaisu | Docker Compose, nginx, Cloudflare Tunnel → https://biometrics.tonikiuru.com |

## Kehitysympäristö

Vaatimukset: Rust (rustup), Node 24+, Docker Desktop.

```bash
# 1. Kehityskanta (PostgreSQL 18, portti 5432 localhostissa)
docker compose -f deploy/docker-compose.dev.yml up -d

# 2. Ympäristömuuttujat
cp deploy/.env.example deploy/.env    # täytä arvot; dev-kannan salasana on "polar", COOKIE_SECURE=false
# Ensimmäisellä käynnistyksellä backend luo omistajakäyttäjän ADMIN_EMAIL/ADMIN_PASSWORD-muuttujista.

# 3. Backend (ajaa migraatiot käynnistyessä, kuuntelee 127.0.0.1:8787)
cd backend && cargo run

# 4. Frontend (Vite dev server, proxyttaa /api backendille)
cd frontend && npm install && npm run dev
```

`sqlx::query!`-makrot tarkistavat SQL:n käännösaikana; siksi `backend/.env` sisältää `DATABASE_URL`-rivin dev-kantaan (git-ignoroitu).

Demo-data ilman Polar-tunnuksia (kun backend on kerran käynnistetty ja omistaja luotu):

```bash
docker exec -i polar-data-hub-dev-db-1 psql -U polar -d polar < backend/scripts/demo_seed.sql
```

Tarkistus: http://localhost:8787/api/health palauttaa `{"status":"ok","database":"up",...}`.

## Polar-tunnukset

1. Luo asiakas osoitteessa https://admin.polaraccesslink.com. Redirect URL kehityksessä on
   `http://localhost:5173/api/polar/callback` (Vite-proxyn kautta, jotta istuntocookie kulkee mukana),
   tuotannossa `https://biometrics.tonikiuru.com/api/polar/callback`. Kumpaakin varten tarvitaan oma asiakas.
2. Kirjoita `POLAR_CLIENT_ID`, `POLAR_CLIENT_SECRET` ja `POLAR_REDIRECT_URL` tiedostoon `deploy/.env`.
3. Kirjaudu sovellukseen omistajana ja avaa `/api/polar/connect` (asetussivu tekee tämän). Token
   tallennetaan kantaan AES-256-GCM-salattuna (`APP_ENCRYPTION_KEY`).

Ilman tunnuksia palvelin käynnistyy normaalisti, mutta yhdistäminen palauttaa 503.

## Rajapinta

Backend julkaisee OpenAPI 3 -kuvauksen osoitteessa `/api/openapi.json` ja Swagger UI:n osoitteessa
`/api/docs`. Lukureitit ovat oletuksena julkisia (`PUBLIC_READ=true`); Polar-yhdistäminen ja
synkronointi vaativat omistajan kirjautumisen. `PUBLIC_READ=false` sulkee kaiken kirjautumisen taakse.

## Komennot

| Mitä | Missä | Komento |
|---|---|---|
| Backend-testit (vaatii dev-kannan) | backend/ | `DATABASE_URL=postgres://polar:polar@127.0.0.1:5432/polar cargo test` |
| Lint | backend/ | `cargo clippy --all-targets -- -D warnings` |
| Formatointi | backend/ | `cargo fmt` |
| Uusi migraatio | backend/ | `sqlx migrate add <nimi>` (ajetaan automaattisesti käynnistyksessä ja testeissä) |
| Offline-kyselydata Docker-buildia varten (aja aina kun SQL-kyselyt muuttuvat, commitoi `.sqlx/`) | backend/ | `cargo sqlx prepare --workspace` |
| Frontend-testit | frontend/ | `npm test` |
| Tyyppitarkistus + build | frontend/ | `npm run build` |
| API-tyyppien generointi | frontend/ | `npm run gen:api` |

## Rakenne

```
backend/    Cargo workspace: crates/api (palvelin), crates/polar-client, crates/domain, migrations/
frontend/   Vite + React + TypeScript: src/pages (sivut), src/components (kaaviot ym.), src/api (client, hooks, generoidut tyypit)
deploy/     docker-compose.yml (tuotanto), docker-compose.dev.yml (kehityskanta), .env.example
docs/       suunnitelma ja kurssimateriaali
```
