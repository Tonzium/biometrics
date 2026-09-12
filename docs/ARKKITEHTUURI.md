# Arkkitehtuuri ja päätökset

Tämä dokumentti kuvaa Polar Data Hubin rakenteen sellaisena kuin se on toteutettu, sekä
keskeiset suunnittelupäätökset perusteluineen (ADR-tyyliin). Suunnitelma ja vaiheistus ovat
tiedostossa [SUUNNITELMA.md](SUUNNITELMA.md), julkaisu tiedostossa [JULKAISU.md](JULKAISU.md).

## 1. Kokonaiskuva

```
                      ┌────────────────────────── kotipalvelin pve2 (Docker Compose) ──────────────────────────┐
                      │                                                                                        │
selain ──HTTPS──► Cloudflare ──tunneli──► cloudflared ──► web (nginx) ──/api/*──► api (Rust/axum) ──► db (Postgres 18)
                      │                                     │  React-build             │                         │
                      │                                     └─ / → index.html           └──HTTPS──► Polar AccessLink API
                      └────────────────────────────────────────────────────────────────────────────────────────┘
```

Neljä konttia, yksi verkko, ei julkaistuja portteja. Ainoa yhteys ulkomaailmaan on cloudflaredin
lähtevä tunneli ja api-kontin lähtevät HTTPS-kutsut Polariin.

| Kerros | Toteutus | Vastuu |
|---|---|---|
| Frontend | React 19, TypeScript 5.9, Vite 8, TanStack Query, react-router, Recharts | Näkymät, tilanhallinta, kaaviot. Ei liiketoimintalogiikkaa. |
| Reverse proxy | nginx:alpine | Staattinen build, `/api` backendille, gzip, välimuistiotsakkeet, SPA-fallback |
| Backend | Rust 1.98, axum 0.8, sqlx 0.9, reqwest 0.13, utoipa | REST-rajapinta, autentikointi, Polar-integraatio, synkronointi, OpenAPI |
| Tietokanta | PostgreSQL 18 | Pysyvä tallennus, näkymät yhteenvetoja varten |
| Julkaisu | Docker Compose, GitHub Actions, GHCR, Cloudflare Tunnel | Buildit CI:ssä, palvelin vetää imaget, TLS Cloudflaressa |

## 2. Backendin rakenne

Cargo-workspace kolmella cratella:

```
backend/crates/
├── domain/        User, Role, DomainError. Ei riippuvuuksia HTTP:hen tai kantaan.
├── polar-client/  Polar AccessLink -asiakas: OAuth2, users, data-reitit, mallit, ISO 8601 -kestot.
│                  Kaikki URL:t konfiguroitavia → testit ajetaan wiremock-mock-palvelinta vasten.
└── api/           axum-palvelin
    ├── config.rs      ympäristömuuttujat → Config (kaatuu heti, jos jokin puuttuu)
    ├── state.rs       AppState: pool, cipher, polar-client, sync-lukko
    ├── auth/          argon2id, JWT, CurrentUser- ja ReadAccess-ekstraktorit, cookie-apurit
    ├── crypto.rs      AES-256-GCM Polar-tokenille
    ├── db/            sqlx::query!-kyselyt tauluittain (käännösaikana tarkistetut)
    ├── sync/          synkronointimoottori ja ajastin
    ├── routes/        health/meta, auth, polar, sync, data (records, summary) + OpenAPI-kokoaminen
    ├── seed.rs        omistajakäyttäjän luonti ensimmäisellä käynnistyksellä
    └── main.rs        ohut käynnistin; kaikki logiikka on lib.rs:n takana testejä varten
```

Reitit jakautuvat kolmeen luokkaan:

| Luokka | Suojaus | Reitit |
|---|---|---|
| Julkiset | ei mitään | `/api/health`, `/api/meta`, `/api/auth/login`, `/api/openapi.json`, `/api/docs` |
| Lukureitit | `ReadAccess`: vapaa, kun `PUBLIC_READ=true`, muuten istunto | `/api/exercises*`, `/api/sleep`, `/api/recharge`, `/api/activity`, `/api/cardio-load`, `/api/physical`, `/api/summary/*` |
| Omistajan reitit | `CurrentUser` + `require_owner()` | `/api/polar/*`, `/api/sync*`, `/api/auth/me`, `/api/auth/logout` |

Suojaus on ekstraktorissa, ei middlewaressa: reitti, joka ottaa parametrina `CurrentUser`-tyypin,
ei voi unohtaa tarkistusta, koska tyyppi ei synny ilman kelvollista istuntoa.

## 3. Tietomalli

```
app_users 1 ──── 0..1 polar_accounts 1 ──── * exercises
                        │                ├── * sleep_nights        (PK account, date)
                        │                ├── * nightly_recharge    (PK account, date)
                        │                ├── * daily_activity      (PK account, date)
                        │                ├── * physical_info       (PK account, date)
                        │                ├── * cardio_load         (PK account, date)
                        └── * sync_runs
```

- Jokaisessa Polar-taulussa on purettu sarakemuoto kyselyjä varten **ja** `raw jsonb`, johon
  Polarin vastaus tallennetaan sellaisenaan. Jos myöhemmin halutaan kenttä, jota ei purettu, se on
  jo kannassa.
- Upsertit (`ON CONFLICT ... DO UPDATE`) tekevät synkronoinnista idempotentin: sama ajo voidaan
  toistaa milloin tahansa ilman duplikaatteja.
- Näkymät `v_daily_wellness` (uni + palautuminen + aktiivisuus + kuorma per päivä) ja
  `v_weekly_summary` (harjoittelu ja uni per viikko) korvaavat vanhan projektin dbt-mallit.
- Migraatiot ovat SQL-tiedostoja `backend/migrations/`, ja backend ajaa ne käynnistyessään.

## 4. Keskeiset virrat

**Julkinen luku.** Selain → nginx → `GET /api/summary/overview`. `ReadAccess` päästää läpi,
kysely hakee ensimmäisen Polar-tilin datan, vastauksesta on jätetty pois tunnisteet ja raaka JSON.

**Kirjautuminen.** `POST /api/auth/login` → argon2id-vertailu (myös tuntemattomalle
sähköpostille valetiivistettä vasten, jotta vasteaika ei paljasta käyttäjän olemassaoloa) → JWT
(HS256, 7 vrk) httpOnly-cookieen `pdh_session`, `SameSite=Lax`, `Secure` tuotannossa.
Rinnakkaisia vertailuja on enintään `LOGIN_MAX_CONCURRENT` (2, tokion `Semaphore`): yksi vertailu
maksaa release-buildissa ~10 ms CPU-aikaa ja varaa 19 MiB muistia, ja koska laskenta ajetaan tokion
blokkaavassa säiepoolissa (512 paikkaa), rajaton tulva yrittäisi varata luokkaa 9 GiB muistia.
Rajan täyttyessä pyyntö hylätään heti 429:llä ja `Retry-After`-otsakkeella ennen kantakyselyä.
Mitattu 40 rinnakkaisella pyynnöllä: 2 vertailua ajettiin, 38 hylättiin ~10 ms:ssa ilman laskentaa.
Permit siirtyy `spawn_blocking`-tehtävään: katkaistu yhteys ei saa vapauttaa sitä ennen kuin
laskenta on todella ohi, koska blokkaavaa tehtävää ei voi keskeyttää.

**Polar-yhdistys (OAuth2 authorization code).**
1. `GET /api/polar/connect` (omistaja): satunnainen `state` 10 min cookieen, 303 → flow.polar.com.
2. Käyttäjä hyväksyy Polarissa. Polar ohjaa selaimen `GET /api/polar/callback?code&state`.
3. Backend vertaa `state`n cookieen, vaihtaa koodin tokeniin (Basic auth client id/secret),
   rekisteröi käyttäjän (`POST /v3/users`, 409 = jo rekisteröity), salaa tokenin AES-256-GCM:llä
   ja tallentaa sen, 303 → `/settings?polar=connected`.
Palvelin ei tarvitse selainta eikä sisääntulevaa yhteyttä Polarilta.

**Synkronointi.** Manuaalisesti `POST /api/sync` tai ajastimesta (`SYNC_INTERVAL_HOURS`).
Yksi ajo kerrallaan (tokio `Mutex::try_lock`). Kuusi askelta (exercises, sleep, nightly-recharge,
activities, physical-info, cardio-load); askeleen virhe kirjataan ja jatketaan, Polarin 429
keskeyttää. Tulos `sync_runs`-tauluun: `ok` / `partial` / `failed`, määrät ja virheet.
Yksittäinen jäsentymätön alkio ohitetaan varoituksella eikä kaada erää, mutta jos koko erästä ei
tallennu yhtään riviä, se kirjataan virheeksi ja ajon tila on `partial`. Varoitus sisältää raa'an
alkion katkaistuna, jotta jäsennysvirheen syy näkyy lokista ilman arvailua.

## 5. Tietoturva

| Aihe | Ratkaisu |
|---|---|
| Salasanat | argon2id (PHC-merkkijono), laskenta `spawn_blocking`-säikeessä |
| Kirjautumistulva | kaksi kerrosta: nginx rajaa kirjautumiset 10/min per IP (`limit_req`, kohdistus `map`-muuttujalla) ja sovellus sallii enintään 2 rinnakkaista salasanatarkistusta (`Semaphore`). Molemmat vastaavat 429 + `Retry-After`. IP-kohtainen raja pitää yhden lähteen tulvan pois muiden tieltä; rinnakkaisuusraja on viimeinen suoja CPU:lle ja muistille myös hajautetussa tulvassa |
| Istunto | JWT httpOnly-cookiessa; JS ei näe sitä. `SameSite=Lax` estää cross-site POSTin (CSRF), mutta sallii OAuth-paluuohjauksen (top-level GET). Jokainen suojattu pyyntö hakee käyttäjän kannasta ja vertaa tokenin `ver`-kenttää `token_version`-sarakkeeseen, joten istunnot voi mitätöidä palvelimelta ja poistetun käyttäjän token lakkaa toimimasta heti |
| OAuth CSRF | `state`-parametri satunnaisesta 32 tavusta, verrataan cookieen, cookie poistetaan aina |
| Polar-token levossa | AES-256-GCM, avain `APP_ENCRYPTION_KEY`, nonce tallennetaan salatekstin eteen |
| SQL-injektio | kaikki kyselyt parametrisoituja `sqlx::query!`-makroja; ainoa dynaaminen SQL on testeissä ja merkitty `AssertSqlSafe` |
| Salaisuudet | vain `.env`-tiedostossa (git-ignoroitu, `chmod 600`); ei koodissa, ei imageissa, ei lokeissa |
| Roolit | `owner` saa yhdistää ja synkronoida; `viewer` vain lukee. Rekisteröintiä ei ole. |
| Julkinen data | vastauksista poistettu Polar-käyttäjä-id, laite-id:t, tilien id:t, raaka JSON. GPS-reittejä ei tuoda kantaan. Paino ja pituus näytetään vain kirjautuneille (`PUBLIC_BODY_METRICS=false`, oletus); backend palauttaa ne `null`-arvoina, joten data ei lähde palvelimelta. |
| Selainotsakkeet | nginx lisää jokaiseen vastaukseen CSP:n (`script-src 'self'`, ei inline-skriptejä), HSTS:n, `nosniff`in, `X-Frame-Options: DENY`in, Referrer- ja Permissions-Policyn sekä COOP/CORP:n (`frontend/security-headers.conf`) |
| HTTPS | nginx ohjaa 301:llä https:ään, jos `CF-Visitor` kertoo selaimen tulleen http:llä. Origin näkee tunnelista aina HTTP:n, joten `$scheme` ei kerro mitään. Tämä on varmistus Cloudflaren "Always Use HTTPS" -asetukselle, ei sen korvike |
| Ympäristömuuttujat | api saa vain nimetyllä listalla olevat muuttujat (ei `env_file`), joten tunnelin token ei ole api-prosessin ympäristössä; CI tarkistaa, ettei lista pääse vanhenemaan |
| Toimitusketju | CI:n actionit kiinnitetty commitin SHA:han, työnkulun oletusoikeus `contents: read`, `cargo audit` ja `npm audit` putkessa, Dependabot päivittää riippuvuudet |
| Verkko | ei avoimia portteja; TLS Cloudflaressa |
| Konttien oikeudet | kaikilla `no-new-privileges`; api ja cloudflared `cap_drop: ALL`; web pudottaa kaikki paitsi neljä nginxin tarvitsemaa. api-prosessi ajaa ei-root-käyttäjänä (uid 10001) |
| Kannan oikeudet | sovellus yhdistää omalla roolilla ilman superuser-oikeuksia (`DB_APP_USER`): ei `COPY ... FROM PROGRAM`, ei palvelimen tiedostoja, ei roolien luontia. Rooli omistaa public-skeeman, jotta migraatiot toimivat; `POSTGRES_USER` jää ylläpitoon |
| Varmuuskopiot | päivittäinen `pg_dump` paikallisesti (`chmod 600`, 30 vrk) ja salattuna koneen ulkopuolelle (`age`, julkinen avain, 90 vrk). Palvelin ei voi purkaa omia etäkopioitaan |
| Rajoitus | `PUBLIC_READ=false` tai Cloudflare Access sulkee sivuston kirjautumisen taakse |

Katselmoinnin löydökset, tehdyt korjaukset ja avoimet kohdat: [TIETOTURVA.md](TIETOTURVA.md).

Tietoisesti tekemättä: refresh-tokenit (7 vrk istunto riittää yhdelle käyttäjälle),
Polar-webhookit (ajastin riittää), täysin ei-root nginx (`nginx-unprivileged` kuuntelisi porttia
8080, mikä vaatisi muutoksen myös Cloudflaren tunnelin kohteeseen) ja oma ei-superuser-rooli
kannassa (kyselyt ovat kaikki parametrisoituja, joten tämä olisi vain syvyyssuuntaista suojaa).
Cloudflaren päässä tehtäväksi jää "Always Use HTTPS" ja halutessa rate limiting -sääntö, joka
pysäyttää tulvan jo ennen originia.

## 6. Päätökset (ADR)

### ADR-1: Rust + axum backendiin Node/Pythonin sijaan
Kurssilla käytettiin Node/Expressiä ja Flask/FastAPI:a. Valittiin Rust, koska tavoitteena oli
oppia tyyppiturvallinen backend ja saada käännösaikainen varmuus SQL-kyselyistä (sqlx). Hinta:
pidemmät käännösajat ja kirjastojen API-muutosten selvittely (argon2 0.6, jsonwebtoken 11,
reqwest 0.13, aes-gcm 0.11 vaativat kaikki tarkistuksen dokumentaatiosta). Hyöty: koko
ajonaikainen virhejoukko "kenttä puuttuu / tyyppi väärin" poistui, ja 59 testiä kattavat kaiken
mock-Polaria vasten.

### ADR-2: PostgreSQL MariaDB:n sijaan
`jsonb` raakavastauksille, `ON CONFLICT DO UPDATE` upserteille, näkymät yhteenvetoihin.
Postgres 18 -imagen datahakemisto on `/var/lib/postgresql` (muuttui 17:stä), mikä on huomioitu
composessa.

### ADR-3: Käännösaikana tarkistetut kyselyt ja `.sqlx`-offline-data
`sqlx::query!` tarkistaa SQL:n kantaa vasten käännöksessä. Jotta Docker-build ja CI eivät tarvitse
kantaa, kyselymetadata generoidaan komennolla `cargo sqlx prepare` ja commitoidaan. CI kääntää
`SQLX_OFFLINE=true`, jolloin vanhentunut metadata kaataa putken näkyvästi. Opittu kantapään
kautta: myös CI:n testivaihe tarvitsee offline-tilan, koska palvelukanta on tyhjä.

### ADR-4: JWT httpOnly-cookiessa, ei Authorization-otsakkeessa
Cookie kulkee automaattisesti myös OAuth-paluuohjauksessa ja täydessä sivunavigoinnissa
(`/api/polar/connect` on tavallinen linkki). Bearer-token localStoragessa olisi altis XSS:lle ja
vaatisi frontendiin token-käsittelyä. `SameSite=Lax` antaa CSRF-suojan ilman erillistä tokenia.

### ADR-5: Julkinen näyteikkuna, suojatut omistajan toiminnot
Sivuston tarkoitus on näyttää omaa dataa. Lukureitit ovat avoimia (`PUBLIC_READ`), vain
Polar-yhdistys ja synkronointi vaativat kirjautumisen. Rajoittaminen myöhemmin on yhden
ympäristömuuttujan tai yhden Cloudflare Access -säännön asia. Kurssin autentikointi- ja
autorisointivaatimus täyttyy roolipohjaisesti.

### ADR-6: Cloudflare Tunnel Let's Encrypt + avoimen portin sijaan
Kotipalvelimelle ei tarvitse avata portteja eikä hoitaa sertifikaattien uusintaa. Kurssin
Certbot-malli on dokumentoitu materiaalissa, mutta tunneli on kotiverkkoon turvallisempi ja
yksinkertaisempi. Haitta: riippuvuus Cloudflaresta.

### ADR-7: Imaget rakennetaan CI:ssä ja julkaistaan GHCR:ään
Rust-käännös tarvitsee 2–4 GB muistia, mitä LXC-kontilla ei välttämättä ole. CI rakentaa imaget
ja palvelin vain vetää ne (`docker compose pull`). Sivuhyöty: versiointi commitin sha:lla ja
paluu edelliseen versioon `IMAGE_TAG`-muuttujalla.

### ADR-8: Sync-moottorin upsert-askeleet traitin, ei sulkeumien kautta
Ensimmäinen toteutus antoi upsert-funktion async-sulkeumana. Tokion `spawn` vaatii `Send`-
futuren, eikä sulkeuman palauttaman futuren `Send`-ominaisuutta voi ilmaista tyyppijärjestelmässä
("implementation of Send is not general enough"). Ratkaisu: `Upsertable`-trait, jonka
`fn upsert(...) -> impl Future<Output = _> + Send` sitoo jokaisen mallityypin omaan
upsert-funktioonsa.

### ADR-9: Vite-proxy ja sama origin kehityksessä
Dev-frontend (5173) proxyttaa `/api`-kutsut backendille (8787). Cookie on silloin same-origin,
CORS-asetuksia ei tarvita, ja Polarin dev-redirect osoittaa Vite-proxyyn
(`http://localhost:5173/api/polar/callback`), jotta istuntocookie kulkee paluuohjauksessa.
Portti 8787 valittiin, koska 8080 kuuluu Windowsissa Hyper-V:n varaamaan alueeseen.

## 7. Testausstrategia

| Taso | Työkalu | Kattaa |
|---|---|---|
| Yksikkö (Rust) | `cargo test` | salasanat, JWT, salaus, ISO 8601 -kestot, mallien jäsennys |
| polar-client | wiremock | OAuth-vaihto, rekisteröinti, datareitit, 429, virheellinen vastausmuoto |
| Integraatio (api) | `#[sqlx::test]` + `tower::oneshot` | jokainen testi saa oman väliaikaisen kannan migraatioineen; reitit kutsutaan ilman verkkoa; Polar mockattu |
| Frontend | Vitest + Testing Library | muotoilut, komponentit, kirjautumislomake ja harjoituslista mock-fetchillä |
| Selain | manuaalinen katselmus demo-datalla | koko käyttöpolku, ks. `backend/scripts/demo_seed.sql` |
| CI | GitHub Actions | fmt, clippy `-D warnings`, kaikki testit, typecheck, lint, build, Docker-imaget, compose-validointi |

## 8. Tunnetut rajoitteet ja jatkokehitys

- Polar antaa historiaa vain noin 28–30 päivää taaksepäin; kanta karttuu vasta ajan myötä.
- Yksi Polar-tili per sovellus näkyy julkisesti (ensimmäinen rekisteröity). Skeema tukee useampaa.
- Harjoitusten näytteet (syke sekunneittain) ja GPS-reitit eivät ole mukana. Ne olisivat
  `GET /v3/exercises/{id}/samples` ja `/gpx`, ja reitit kannattaisi pitää kirjautumisen takana.
- Polar-webhookit (`POST /v3/webhooks`) tekisivät synkronoinnista välittömän; julkinen osoite on
  olemassa, joten tämä on luonteva jatko.
- Domain on `biometrics.tonikiuru.com`, koska myöhemmin voidaan lisätä muita datalähteitä.
  Sovelluksen nimi "Polar Data Hub" on vielä Polar-keskeinen.
