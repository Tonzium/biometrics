# Polar Data Hub – lopputyön suunnitelma

**Kurssi:** Web-sovelluskehitys (KAMK), Full stack
**Tekijä:** Toni Kiuru
**Päivämäärä:** 10.9.2026
**Tuotanto-osoite:** https://biometrics.tonikiuru.com

## 1. Tavoite

Rakennetaan kokonainen full stack -web-sovellus, joka

1. hakee omat harjoitus-, uni-, palautumis- ja aktiivisuustiedot Polar Flow -kellosta Polar AccessLink API:n kautta,
2. tallentaa ne pysyvästi omaan PostgreSQL-tietokantaan (Polar säilyttää dataa rajapinnassa vain 28–30 päivää, joten oma kanta on ainoa tapa kerryttää pitkä historia),
3. tarjoaa REST-rajapinnan ja React/TypeScript-käyttöliittymän datan katselmointiin, ja
4. julkaistaan Docker-konteissa Nginxin taakse ja internetiin Cloudflare Tunnelin kautta ilman yhtään avointa porttia palvelimella.

Vanha projekti [Tonzium/polar-data-analysis](https://github.com/Tonzium/polar-data-analysis) (Python + DuckDB + dbt) toimii ideatason esikuvana: samat Polar-datalähteet, sama "oma data omaan haltuun" -ajatus. Toteutus tehdään alusta asti uudelleen eri teknologioilla.

## 2. Teknologiavalinnat

Kurssilla opetettiin Node/Express-, Python/Flask/FastAPI- ja MariaDB-pino. Tässä työssä samat käsitteet (REST, autentikointi, relaatiokanta, Docker, Nginx, testaus) toteutetaan Rustilla, TypeScriptillä ja PostgreSQL:llä.

### Versiot (tarkistettu 10.9.2026)

| Osa | Valinta | Versio |
|---|---|---|
| Kieli, backend | Rust, edition 2024 | 1.98.1 stable |
| HTTP-kehys | axum | 0.8 |
| Async-runtime | tokio | 1.53 |
| Tietokanta-ajuri | sqlx (postgres, migrations, compile-time-tarkistetut kyselyt) | 0.9 |
| HTTP-asiakas (Polar API) | reqwest (rustls) | 0.13 |
| Serialisointi | serde, serde_json | 1.0 |
| OpenAPI-dokumentaatio | utoipa + utoipa-swagger-ui | 5 |
| Autentikointi | jsonwebtoken 11, argon2 0.6 | |
| Middleware | tower-http (cors, trace, compression) | 0.7 |
| Lokitus | tracing, tracing-subscriber | 0.1 |
| Aika/ID | chrono 0.4, uuid 1 | |
| Tietokanta | PostgreSQL | 18 |
| Kieli, frontend | TypeScript (strict) | 5.9 (openapi-typescript ei vielä tue TS 6/7:ää) |
| UI-kirjasto | React | 19 |
| Bundleri | Vite | 8 |
| Reititys | react-router | 8 |
| Datan haku/välimuisti | TanStack Query | 5 |
| Kaaviot | Recharts | 3 |
| Testit, backend | cargo test, `#[sqlx::test]` | |
| Testit, frontend | Vitest + React Testing Library | |
| Kontit | Docker 29, Docker Compose v5 | |
| Reverse proxy | nginx:alpine | |
| Julkaisu | cloudflared (Cloudflare Tunnel) | |

### Perustelut

- **Rust + axum + sqlx:** tyyppiturvallinen backend, jossa SQL-kyselyt tarkistetaan käännösaikana kantaa vasten. Vastaa kurssin Express/FastAPI-osiota, mutta ilman ajonaikaisia yllätyksiä.
- **PostgreSQL:** kurssilla käytettiin MariaDB:tä. Postgresin `jsonb` sopii Polar-API:n raakavastausten tallentamiseen, ja `ON CONFLICT ... DO UPDATE` tekee toistuvasta synkronoinnista idempotentin.
- **OpenAPI → TypeScript:** backend generoi OpenAPI-kuvauksen (utoipa), josta frontendin API-tyypit generoidaan `openapi-typescript`-työkalulla. Rajapinta pysyy yhdenmukaisena molemmissa päissä.
- **Cloudflare Tunnel:** palvelimelle ei avata portteja 80/443 eikä tarvita Let's Encrypt/Certbot-kikkailua. TLS päätetään Cloudflaressa ja tunneli on salattu.

## 3. Arkkitehtuuri

```
Selain ──HTTPS──> Cloudflare edge ──tunneli──> cloudflared-kontti
                                                    │
                                                    ▼
                                             nginx (web-kontti)
                                       /        → staattinen React-build
                                       /api/*   → proxy_pass http://api:8080
                                                    │
                                                    ▼
                                             api (Rust/axum)
                                        ┌───────────┴───────────┐
                                        ▼                       ▼
                                 PostgreSQL (db)      Polar AccessLink API v3
                                                      (OAuth2 + REST, ulos internetiin)
```

- Yksikään kontti ei julkaise portteja isäntäkoneelle tuotannossa. Ainoa yhteys ulos on cloudflaredin avaama lähtevä tunneli.
- `api`-kontti ajaa myös **synkronointiajastimen** (tokio-taustatehtävä): ensimmäinen ajo minuutti käynnistyksestä, sitten `SYNC_INTERVAL_HOURS` välein (oletus 6 h, 0 = pois). Lisäksi käyttöliittymässä on "Synkronoi nyt" -nappi. Vain yksi ajo kerrallaan; jokainen ajo kirjataan `sync_runs`-tauluun. Polarin 429 keskeyttää ajon, muut virheet merkitään `partial`-tilaan ja loput datatyypit haetaan silti.
- Kehityksessä erillinen `docker-compose.dev.yml` ajaa vain kannan (portti 5432 localhostissa). Backend ajetaan `cargo run` portissa 8787 (8080 kuuluu Windowsissa Hyper-V:n varaamaan alueeseen) ja Vite dev server proxyttaa `/api` sinne.

### Repon rakenne (monorepo)

```
Lopputyo/
├── backend/                  Cargo workspace
│   ├── Cargo.toml
│   ├── crates/
│   │   ├── api/              axum-palvelin, reitit, auth, sync-ajastin (binary)
│   │   ├── polar-client/     AccessLink OAuth2 + REST-asiakas (library)
│   │   └── domain/           tietomallit, DTO:t, virhetyypit (library)
│   ├── migrations/           sqlx-migraatiot (SQL)
│   ├── .sqlx/                offline-kyselymetadata Docker-buildia varten
│   └── Dockerfile            monivaiheinen build
├── frontend/
│   ├── src/
│   │   ├── api/              generoidut tyypit + fetch-wrapperit
│   │   ├── components/
│   │   ├── pages/
│   │   └── hooks/
│   ├── nginx.conf
│   ├── Dockerfile            build-vaihe (node) + nginx-vaihe
│   └── vite.config.ts
├── deploy/
│   ├── docker-compose.yml
│   ├── docker-compose.dev.yml
│   └── .env.example
├── docs/
│   ├── Guide.md              kurssimateriaali
│   ├── SUUNNITELMA.md        tämä tiedosto
│   ├── ARKKITEHTUURI.md      rakenne, virrat, tietoturva, ADR:t
│   ├── JULKAISU.md           palvelinasennus askel askeleelta
│   └── RAPORTTI.md           kurssiraportti (OpenAPI-kuvaus on ajossa /api/docs)
├── .github/workflows/ci.yml
└── README.md
```

Repon nykyinen Python-runko (`pyproject.toml`, `.python-version`, `src/lopputyo/`) poistetaan, koska se on uv:n luoma oletuspohja eikä liity toteutukseen.

## 4. Tietokantasuunnitelma

Kaikkiin Polar-tauluihin tallennetaan sekä purettu sarakemuoto (kyselyitä varten) että `raw jsonb` (alkuperäinen vastaus). Synkronointi tekee upsertin luonnolliseen avaimeen.

| Taulu | Avain | Keskeiset sarakkeet | Lähde |
|---|---|---|---|
| `app_users` | `id uuid` | email, password_hash (argon2), role, created_at | oma |
| `polar_accounts` | `id uuid` | app_user_id FK, polar_user_id, access_token (salattu levossa), registered_at, last_sync_at | OAuth |
| `exercises` | `polar_exercise_id text` | polar_account_id FK, start_time, duration, sport, distance, calories, hr_avg, hr_max, training_load, device, raw | `GET /v3/exercises`, `/v3/exercises/{id}` |
| `sleep_nights` | (`polar_account_id`, `date`) | sleep_start, sleep_end, light/deep/rem s, sleep_score, sleep_charge, interruptions, raw | `GET /v3/users/sleep` |
| `nightly_recharge` | (`polar_account_id`, `date`) | hr_avg, hrv_avg, breathing_rate_avg, recharge_status, ans_charge, raw | `GET /v3/users/nightly-recharge` |
| `daily_activity` | (`polar_account_id`, `date`) | steps, calories, active_calories, active_duration, raw | `GET /v3/users/activities` (uusi listamalli, ei transaktioita) |
| `physical_info` | (`polar_account_id`, `date`) | modified_at, weight_kg, height_cm, maximum/resting_heart_rate, vo2_max, raw | `GET /v3/users/physical-info` (palauttaa nykytilan; historia kertyy `modified`-päivän mukaan) |
| `cardio_load` | (`polar_account_id`, `date`) | status, cardio_load, strain, tolerance, cardio_load_ratio, raw | `GET /v3/users/cardio-load` |
| `sync_runs` | `id identity` | polar_account_id, trigger, status, started_at, finished_at, counts jsonb, error | oma loki |

Lisäksi SQL-näkymät kaavioita varten: `v_weekly_summary` (viikoittainen harjoitusmäärä, kesto, kuorma, keskimääräinen unipiste) ja `v_daily_wellness` (uni + palautuminen + aktiivisuus per päivä). Nämä korvaavat vanhan projektin dbt-mallit.

## 5. Backend-rajapinta

Kaikki reitit `/api`-etuliitteellä. **Lukureitit ovat julkisia** (näyteikkuna; `PUBLIC_READ=false` sulkee ne kirjautumisen taakse), **omistajan reitit** (Polar-yhdistäminen, synkronointi) vaativat kirjautumisen. Autentikointi JWT:llä httpOnly-cookiessa (`pdh_session`, SameSite=Lax, Secure tuotannossa). Suojattu reitti ottaa parametrina `CurrentUser`-ekstraktorin, joka tarkistaa tokenin ennen reitin koodia. Kirjautumisvirhe on aina 401 erottelematta tuntematonta sähköpostia väärästä salasanasta, ja salasanan tarkistus ajetaan myös tuntemattomalle käyttäjälle ajoitushyökkäysten varalta. Dokumentaatio `/api/docs` (Swagger UI).

| Metodi | Polku | Kuvaus |
|---|---|---|
| GET | `/api/health` | Tila + kantayhteys |
| POST | `/api/auth/login` | Kirjautuminen, asettaa cookien |
| POST | `/api/auth/logout` | |
| GET | `/api/auth/me` | Kirjautunut käyttäjä |
| GET | `/api/polar/status` | Omistaja: onko Polar-tunnukset asetettu, onko tili yhdistetty, viimeisin synkronointi |
| GET | `/api/polar/connect` | Omistaja: ohjaa Polar OAuth2 -valtuutukseen. Satunnainen `state` talletetaan 10 min httpOnly-cookieen (CSRF-suoja) |
| GET | `/api/polar/callback` | Omistaja: tarkistaa `state`n, vaihtaa koodin tokeniin, rekisteröi käyttäjän (`POST /v3/users`, 409 = jo rekisteröity), tallentaa salatun tokenin, ohjaa `/settings?polar=connected|denied|error` |
| DELETE | `/api/polar/disconnect` | Omistaja: poistaa rekisteröinnin Polarista (`DELETE /v3/users/{id}`) ja tilin kannasta |
| POST | `/api/sync` | Omistaja: ajaa synkronoinnin heti ja palauttaa raportin (status ok/partial/failed, määrät, virheet). 409 jos ajo on jo käynnissä |
| GET | `/api/sync/runs?limit` | Omistaja: viimeisimmät ajot `sync_runs`-taulusta |
| GET | `/api/meta` | Julkinen: `public_read`, `polar_configured`, versio (frontend päättää tästä, näyttääkö kirjautumisen ensin) |
| GET | `/api/exercises?from&to&sport&page&per_page` | Julkinen: sivutettu lista uusin ensin, `{items, page, per_page, total}` |
| GET | `/api/exercises/{id}` | Julkinen: yksittäinen harjoitus sykevyöhykkeineen |
| GET | `/api/sleep?from&to` | Julkinen: yöt (oletus 30 pv, max 366) hypnogrammeineen |
| GET | `/api/recharge?from&to` | Julkinen: Nightly Recharge |
| GET | `/api/activity?from&to` | Julkinen: päiväaktiivisuus |
| GET | `/api/cardio-load?from&to` | Julkinen: cardio load |
| GET | `/api/physical` | Julkinen: aikasarja painosta, VO2max:sta, leposykkeestä |
| GET | `/api/summary/overview` | Julkinen: määrät, aikaväli, lajit, tuoreimmat arvot, viimeisin synkronointi |
| GET | `/api/summary/daily?from&to` | Julkinen: näkymä `v_daily_wellness` |
| GET | `/api/summary/weekly?weeks=12` | Julkinen: näkymä `v_weekly_summary` |
| GET | `/api/openapi.json`, `/api/docs` | OpenAPI 3 -kuvaus ja Swagger UI |

Julkisista vastauksista on jätetty pois Polar-käyttäjä-id, laite-id:t, tilien id:t ja raaka JSON. Paino ja pituus näkyvät vain kirjautuneille, ellei `PUBLIC_BODY_METRICS=true`.

Virheet palautetaan yhtenäisenä JSON-muotona `{ "error": { "code": "...", "message": "..." } }` ja oikeilla HTTP-koodeilla (400, 401, 404, 409, 429, 500). Polarin 429-vastaukset käsitellään `RateLimit-Reset`-otsakkeen mukaan.

### Polar-asiakas (`polar-client`)

- OAuth2 authorization code -virta käsin reqwestillä (Polarin token-vastaus sisältää epästandardin `x_user_id`-kentän, joten valmis oauth2-crate ei istu suoraan).
- Token ei vanhene, mutta se salataan kantaan sovelluksen avaimella (`APP_ENCRYPTION_KEY`).
- Jokaiselle datatyypille oma tyypitetty vastausrakenne serdellä. Tuntemattomat kentät säilyvät `raw`-sarakkeessa.
- Polar-kehittäjätilille (admin.polaraccesslink.com) tarvitaan kaksi asiakasta, koska redirect URL on kiinteä: `http://localhost:5173/api/polar/callback` kehitykseen (Vite-proxyn kautta, jotta istuntocookie on mukana) ja `https://biometrics.tonikiuru.com/api/polar/callback` tuotantoon.

## 6. Frontend

- Vite + React 19 + TypeScript strict. Ei UI-kirjastoa; yksi `index.css` CSS-muuttujilla, responsiivinen grid.
- Julkinen näyteikkuna: `ReadGuard` päästää datasivuille ilman kirjautumista, kun `/api/meta` kertoo `public_read=true`; `OwnerGuard` suojaa asetussivun aina.
- TanStack Query hoitaa API-kutsut, välimuistin ja lataus/virhetilat. react-router hoitaa sivut ja suojatut reitit.
- Sivut:
  1. **Kirjautuminen**
  2. **Dashboard** – viimeisen 7/30 pv tunnusluvut (harjoitukset, unipisteet, palautuminen, askeleet) ja viikkokaavio
  3. **Harjoitukset** – suodatettava taulukko + harjoituksen yksityiskohdat (syke, kesto, kuorma)
  4. **Uni ja palautuminen** – univaiheet pinottuna palkkikaaviona, HRV-trendi
  5. **Aktiivisuus** – askeleet ja kalorit päivittäin
  6. **Asetukset** – Polar-tilin yhdistäminen/irrotus, "Synkronoi nyt", synkronointiloki
- API-tyypit generoidaan komennolla `npm run gen:api` backendin OpenAPI-kuvauksesta (`src/api/schema.d.ts`, commitoidaan). `src/api/types.ts` antaa niille lyhyet nimet ja `hooks.ts` sitoo ne TanStack Query -hookeiksi.
- Demo-data ilman Polar-tunnuksia: `backend/scripts/demo_seed.sql`.

## 7. Julkaisu

Tuotanto ajetaan kotipalvelimen Proxmox-kontissa `pve2`, johon asennetaan Docker ja Docker Compose. Domain `tonikiuru.com` on Cloudflaren DNS:ssä.

`docker-compose.yml` (tuotanto):

| Palvelu | Image | Portit | Huomiot |
|---|---|---|---|
| `db` | postgres:18-alpine | ei julkaistu | nimetty volume `pgdata`, healthcheck `pg_isready` |
| `api` | `ghcr.io/<owner>/<repo>-api` (CI rakentaa: rust:1.98 → debian-slim) | ei julkaistu | saa vain nimetyllä listalla olevat ympäristömuuttujat (ei `env_file`, joten tunnelin token ei päädy api-prosessiin), `depends_on: db: condition: service_healthy` |
| `web` | `ghcr.io/<owner>/<repo>-web` (CI rakentaa: node:24 → nginx:alpine) | ei julkaistu | staattinen build + `/api` proxy |
| `cloudflared` | cloudflare/cloudflared | ei julkaistu | `tunnel run`, token `.env`:stä; ingress (`biometrics.tonikiuru.com` → `http://web:80`) määritellään Cloudflaren hallintapaneelissa, joten erillistä config.yml:ää ei tarvita |

Tarkka ohje: `docs/JULKAISU.md`. Askeleet lyhyesti:

1. Cloudflare Zero Trust → Networks → Tunnels → luo tunneli, kopioi token.
2. Tunnelin public hostname `biometrics.tonikiuru.com` → `http://web:80`. Cloudflare luo CNAME-tietueen automaattisesti.
3. Cloudflare SSL/TLS-tila "Full" ja "Always Use HTTPS".
4. Valinnainen lisäkerros: Cloudflare Access -sääntö, joka päästää vain omaan sähköpostiin kirjautuneet. Sovelluksen oma kirjautuminen jää silti paikalleen kurssivaatimuksena.
5. Palvelimella: `./deploy/deploy.sh` (git pull, `docker compose pull`, `up -d`). Imaget rakennetaan CI:ssä, joten palvelimen ei tarvitse kääntää Rustia.

Salaisuudet (`.env`, ei koskaan gitiin): `DATABASE_URL`, `JWT_SECRET`, `APP_ENCRYPTION_KEY`, `POLAR_CLIENT_ID`, `POLAR_CLIENT_SECRET`, `POLAR_REDIRECT_URL`, `CLOUDFLARE_TUNNEL_TOKEN`, `ADMIN_EMAIL`, `ADMIN_PASSWORD` (vain ensimmäistä käynnistystä varten).

## 8. Testaus ja CI

- **Backend:** yksikkötestit polar-clientin JSON-purulle (tallennetut esimerkkivastaukset), integraatiotestit reiteille `#[sqlx::test]`-makrolla (jokainen testi saa oman väliaikaisen kannan), `cargo clippy -D warnings`, `cargo fmt --check`.
- **Frontend:** Vitest + React Testing Library komponenteille ja hookeille, `tsc --noEmit`, ESLint.
- **CI (GitHub Actions)** `.github/workflows/ci.yml`: kolme jobia. `backend` ajaa fmt-tarkistuksen, clippyn offline-kyselydatalla (paljastaa vanhentuneen `.sqlx`-kansion) ja testit Postgres 18 -palvelukonttia vasten; `frontend` ajaa typecheckin, lintin, testit ja buildin; `docker` rakentaa molemmat imaget GHA-välimuistilla ja validoi compose-tiedoston.
- Kurssi mainitsee GitLabin; repo on GitHubissa. Jos palautus vaatii GitLabin, sama repo peilataan sinne.

## 9. Vaiheistus

| Vaihe | Sisältö | Valmis kun |
|---|---|---|
| 0. Työkalut ✅ 10.9.2026 | rustup 1.98.1 + VS Build Tools, sqlx-cli 0.9, Node 24, Docker; repon runko; Python-pohjan poisto | `cargo run` ja `npm run dev` käynnistyvät |
| 1. Kanta + runko ✅ 10.9.2026 | migraatiot 0001–0004 (kaikki luvun 4 taulut + näkymät), yhtenäinen virhemuoto, `#[sqlx::test]`-testirunko | health palauttaa kantayhteyden, 3 integraatiotestiä vihreänä |
| 2. Auth ✅ 10.9.2026 | argon2id-tiivisteet, JWT (HS256) httpOnly+SameSite=Lax-cookiessa, `CurrentUser`-ekstraktori, omistajan seed ympäristömuuttujista, login/logout/me | suojattu reitti palauttaa 401 ilman cookieta; 8 integraatiotestiä + 6 yksikkötestiä |
| 3. Polar-yhteys ✅ 10.9.2026 | polar-client (OAuth2, users), AES-256-GCM-salattu token, CSRF-state-cookie, reitit status/connect/callback/disconnect, wiremock-testit | oma Polar-tili näkyy asetuksissa (frontend vaiheessa 6; backend testattu mock-Polaria vasten) |
| 4. Synkronointi ✅ 10.9.2026 | polar-clientin datareitit + tyypitetyt mallit + ISO 8601 -kestot, upsertit kuuteen tauluun, sync_runs-loki, `POST /api/sync`, `GET /api/sync/runs`, ajastin (SYNC_INTERVAL_HOURS), 429 keskeyttää ajon | mock-Polaria vasten: 2 ajoa ei duplikoi rivejä, partial/failed-tilat testattu. Oikea data vaatii Polar-tunnukset `.env`:iin |
| 5. Data-API ✅ 10.9.2026 | julkiset lukureitit (exercises sivutettuna + laji/aikasuodatin, sleep, recharge, activity, cardio-load, physical), summary overview/daily/weekly, `PUBLIC_READ`-lippu ja `ReadAccess`-ekstraktori, `/api/meta`, OpenAPI `/api/openapi.json` + Swagger UI `/api/docs` | 8 uutta integraatiotestiä; tyypit generoituvat frontendiin (`npm run gen:api`) |
| 6. Frontend ✅ 10.9.2026 | Vite 8 + React 19 + TS: yleiskuva, harjoitukset (suodatus, sivutus, yksityiskohdat sykevyöhykkeineen), uni ja palautuminen, aktiivisuus ja kuormitus, kirjautuminen, asetukset (Polar-yhdistys, synkronointi, ajoloki); Recharts-kaaviot; generoidut API-tyypit; nginx.conf + Dockerfile | koko käyttöpolku katselmoitu selaimessa demo-datalla; 12 Vitest-testiä |
| 7. Testit + CI ✅ 11.9.2026 | 59 backend-testiä (yksikkö + `#[sqlx::test]`-integraatio mock-Polaria vasten), 12 frontend-testiä; GitHub Actions: rust (fmt, clippy offline, testit Postgres-palvelukontilla), node (typecheck, lint, test, build), docker (molemmat imaget + compose-validointi); backendin monivaiheinen Dockerfile; `docker-compose.local.yml` tuotantopinon koeajoon ilman tunnelia | CI vihreä GitHubissa 11.9.2026 (kaksi korjausta: testivaihe tarvitsi `SQLX_OFFLINE`, compose-validointi `.env`-tiedoston); pino ajettu paikallisesti konteissa |
| 8. Julkaisu ✅ 11.9.2026 | LXC-kontti pve2:lla, Docker, imaget GHCR:stä, Cloudflare Tunnel; `docs/JULKAISU.md`, `deploy/deploy.sh`, `deploy/backup.sh`. Julkaisussa löytyi kaksi ohjevirhettä (Postgres-salasanan base64-merkit rikkoivat yhteysosoitteen → hex; admin-salasanan vähimmäispituus) ja ne korjattiin ohjeeseen | https://biometrics.tonikiuru.com vastaa; Polar-tunnukset ja ensimmäinen synkronointi seuraavaksi |
| 9. Dokumentointi ✅ 11.9.2026 | README, `ARKKITEHTUURI.md` (rakenne, virrat, tietoturva, 9 ADR:ää, testausstrategia), `JULKAISU.md`, `RAPORTTI.md` (kurssin aiheet → toteutus, AI-käytön kriittinen arviointi; omat pohdinnat merkitty `[TÄYDENNÄ]`) | |

Laajennukset, jos aikaa jää: Polar-webhookit (`POST /v3/webhooks`, mahdollista koska julkinen osoite on olemassa), jatkuva syke `GET /v3/users/continuous-heart-rate/{date}`, harjoituksen FIT/GPX-lataus ja reittikartta, PWA-asennettavuus.

## 10. Kurssin sisältö → toteutus

| Kurssin aihe (Guide.md) | Missä toteutuu |
|---|---|
| Asiakas–palvelin-malli, HTTP/HTTPS | selain → Cloudflare → nginx → axum |
| HTML, CSS, JavaScript/TypeScript | frontend/ |
| Komponenttipohjainen UI, tilanhallinta, API-kutsut | React 19, TanStack Query |
| REST-rajapinnan suunnittelu | luku 5, OpenAPI |
| Autentikointi ja käyttäjähallinta | JWT + argon2, OAuth2 Polaria vastaan |
| Relaatiotietokannan suunnittelu ja SQL | luku 4, sqlx-migraatiot, näkymät |
| Tietokannan integrointi backendiin | sqlx, compile-time-tarkistetut kyselyt |
| Testaus ja virheenkäsittely | luku 8, yhtenäinen virhemuoto |
| Versionhallinta | GitHub (tarvittaessa GitLab-peili) |
| Docker, Docker Compose, monivaiheinen build | deploy/ |
| Nginx reverse proxy ja HTTPS | web-kontti + Cloudflare TLS |
| .env ja salaisuudet | luku 7 |
| AI-työkalut kehityksessä | Claude Code käytössä; kriittinen arviointi dokumentoidaan raporttiin |

## 11. Päätetyt reunaehdot (10.9.2026)

1. **Tuotantopalvelin:** kotipalvelin, Proxmox-kontti `pve2` (kotiverkossa). Konttiin asennetaan Docker + Compose; cloudflared ajetaan konttina samassa compose-pinossa.
2. **DNS:** `tonikiuru.com` on jo Cloudflaressa, joten tunnelin public hostname luo CNAME-tietueen suoraan.
3. **Käyttäjät:** yksi sovelluskäyttäjä. Rekisteröinti suljettu; admin-tunnus luodaan ensimmäisellä käynnistyksellä ympäristömuuttujista. Skeema tukee silti useampaa käyttäjää ja Polar-tiliä.
4. **Polar-kehittäjätili:** olemassa. Luodaan sinne kaksi asiakasta (dev ja prod) eri redirect URL -osoitteilla.
5. **Versionhallinta:** GitHub. Toni tekee commitit ja pushit itse jokaisen milestonen jälkeen.
