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
| Julkaisu | Docker Compose, nginx, Cloudflare Tunnel → https://polar.tonikiuru.com |

## Kehitysympäristö

Vaatimukset: Rust (rustup), Node 24+, Docker Desktop.

```bash
# 1. Kehityskanta (PostgreSQL 18, portti 5432 localhostissa)
docker compose -f deploy/docker-compose.dev.yml up -d

# 2. Ympäristömuuttujat
cp deploy/.env.example deploy/.env    # täytä arvot; dev-kannan salasana on "polar"

# 3. Backend (ajaa migraatiot käynnistyessä, kuuntelee 127.0.0.1:8787)
cd backend && cargo run

# 4. Frontend (Vite dev server, proxyttaa /api backendille)
cd frontend && npm install && npm run dev
```

Tarkistus: http://localhost:8787/api/health palauttaa `{"status":"ok","database":"up",...}`.

## Komennot

| Mitä | Missä | Komento |
|---|---|---|
| Backend-testit (vaatii dev-kannan) | backend/ | `DATABASE_URL=postgres://polar:polar@127.0.0.1:5432/polar cargo test` |
| Lint | backend/ | `cargo clippy --all-targets -- -D warnings` |
| Formatointi | backend/ | `cargo fmt` |
| Uusi migraatio | backend/ | `sqlx migrate add <nimi>` (ajetaan automaattisesti käynnistyksessä ja testeissä) |
| Offline-kyselydata Docker-buildia varten | backend/ | `cargo sqlx prepare --workspace` |
| Frontend-testit | frontend/ | `npm test` |
| Tyyppitarkistus + build | frontend/ | `npm run build` |
| API-tyyppien generointi | frontend/ | `npm run gen:api` |

## Rakenne

```
backend/    Cargo workspace: crates/api (palvelin), crates/polar-client, crates/domain, migrations/
frontend/   Vite + React + TypeScript
deploy/     docker-compose.yml (tuotanto), docker-compose.dev.yml (kehityskanta), .env.example
docs/       suunnitelma ja kurssimateriaali
```
