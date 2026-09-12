# Lopputyöraportti: Polar Data Hub

**Kurssi:** Web-sovelluskehitys (KAMK)
**Tekijä:** Toni Kiuru
**Sovellus:** https://biometrics.tonikiuru.com
**Lähdekoodi:** https://github.com/Tonzium/biometrics

> Kohdat, joissa on `[TÄYDENNÄ]`, ovat omaa pohdintaasi varten. Tekninen sisältö on ajan tasalla.

## 1. Tavoite ja lopputulos

Rakensin full stack -web-sovelluksen, joka hakee omat harjoitus-, uni-, palautumis- ja
aktiivisuustiedot Polar Flow -kellosta Polar AccessLink API:n kautta, tallentaa ne pysyvästi omaan
PostgreSQL-kantaan ja näyttää ne julkisena näyteikkunana. Polar säilyttää dataa rajapinnassaan
vain noin kuukauden, joten oma kanta on ainoa tapa kerryttää pitkä historia. Sovellus on julkaistu
kotipalvelimelle Docker-konteissa ja internetiin Cloudflare Tunnelilla ilman avoimia portteja.

Idean pohjana oli aiempi projektini
[polar-data-analysis](https://github.com/Tonzium/polar-data-analysis) (Python, DuckDB, dbt),
josta otettiin datalähteet ja ajatus, mutta toteutus tehtiin alusta asti uudelleen.

## 2. Teknologiavalinnat ja poikkeama kurssin pinosta

Kurssilla käytettiin Node/Expressiä, Flask/FastAPI:a, Reactia ja MariaDB:tä. Toteutin samat
käsitteet eri välineillä:

| Kurssin väline | Tässä työssä | Miksi |
|---|---|---|
| Node/Express tai FastAPI | Rust 1.98, axum 0.8 | tyyppiturvallisuus, käännösaikana tarkistettu SQL, oppimistavoite |
| MariaDB + mysql2 | PostgreSQL 18 + sqlx 0.9 | jsonb raakadatalle, upsertit, näkymät |
| React (JavaScript) | React 19 + TypeScript 5.9, Vite 8 | tyypit generoidaan backendin OpenAPI-kuvauksesta |
| Nginx + Certbot | nginx + Cloudflare Tunnel | ei avoimia portteja kotiverkkoon, TLS Cloudflaressa |
| Docker Compose | Docker Compose + GitHub Actions + GHCR | imaget rakennetaan CI:ssä, palvelin vetää ne |

Tarkemmat perustelut ovat arkkitehtuuridokumentin ADR-osiossa ([ARKKITEHTUURI.md](ARKKITEHTUURI.md)).

## 3. Kurssin sisältö toteutuksessa

| Kurssin aihe | Missä toteutuu | Todiste |
|---|---|---|
| Asiakas–palvelin-malli, HTTP/HTTPS | selain → Cloudflare → nginx → axum | `deploy/docker-compose.yml`, `frontend/nginx.conf` |
| HTML, CSS, JavaScript/TypeScript | React-komponentit, yksi `index.css` CSS-muuttujilla | `frontend/src/` |
| Komponenttipohjainen UI, tilanhallinta | React 19, TanStack Query, URL-tila suodattimille | `frontend/src/pages/`, `api/hooks.ts` |
| REST-rajapinnan suunnittelu | 21 reittiä, oikeat statuskoodit, yhtenäinen virhemuoto, OpenAPI + Swagger UI | `/api/docs`, `backend/crates/api/src/routes/` |
| Autentikointi ja käyttäjähallinta | argon2id, JWT httpOnly-cookiessa, roolit owner/viewer, OAuth2 Polaria vastaan | `backend/crates/api/src/auth/`, `routes/polar.rs` |
| Relaatiotietokannan suunnittelu ja SQL | 8 taulua, 2 näkymää, 5 migraatiota, upsertit | `backend/migrations/` |
| Tietokannan integrointi backendiin | sqlx, käännösaikana tarkistetut kyselyt, offline-metadata | `backend/crates/api/src/db/`, `.sqlx/` |
| Testaus ja virheenkäsittely | 59 Rust-testiä (yksikkö + integraatio omalla kannalla per testi, Polar mockattu), 12 frontend-testiä; `ApiError` → HTTP-koodit | `backend/crates/api/tests/`, `frontend/src/**/*.test.tsx` |
| Versionhallinta | GitHub, commit per vaihe | commit-historia |
| Docker, monivaiheinen build, Compose | kaksi Dockerfilea, compose + override-tiedostot | `backend/Dockerfile`, `frontend/Dockerfile`, `deploy/` |
| Nginx reverse proxy ja HTTPS | nginx `/api` → api, SPA-fallback; TLS Cloudflaressa | `frontend/nginx.conf`, `docs/JULKAISU.md` |
| .env ja salaisuudet | kaikki salaisuudet `.env`:ssä, git-ignoroitu, Polar-token salattu kannassa | `deploy/.env.example`, `crypto.rs` |
| AI-työkalut kehityksessä | Claude Code parityöskentelynä, ks. luku 5 | tämä raportti |

## 4. Toteutuksen kulku

Työ eteni yhdeksässä vaiheessa, joista jokainen päättyi commitiin ja verifiointiin:

0. Työkalut ja repon runko (rustup, Build Tools, sqlx-cli, Vite-pohja, compose-tiedostot)
1. Kanta ja migraatiot, health-reitti
2. Kirjautuminen (argon2id, JWT-cookie, omistajan seed)
3. Polar-yhdistys (OAuth2, tokenin salaus, polar-client-crate)
4. Synkronointi (datareitit, upsertit, sync_runs-loki, ajastin)
5. Julkiset data- ja summary-reitit, PUBLIC_READ, OpenAPI
6. React-käyttöliittymä
7. CI ja Docker-imaget
8. Julkaisu (Cloudflare Tunnel, GHCR)
9. Dokumentointi

Vaiheistus ja jokaisen vaiheen valmiuskriteeri ovat tiedostossa [SUUNNITELMA.md](SUUNNITELMA.md).

`[TÄYDENNÄ: mikä oli vaikeinta, mihin meni eniten aikaa, mitä tekisit toisin]`

## 5. AI-työkalujen käyttö ja kriittinen arviointi

Kurssimateriaalin mukaan tekoälyä on käytettävä assistenttina, ei arkkitehtina, ja jokainen rivi on
ymmärrettävä. Käytin Claude Codea (Anthropic) parityöskentelyyn koko projektin ajan: suunnittelu,
koodi, testit, dokumentaatio. Työtapa oli Context–Task–Constraints-mallin mukainen: jokaiselle
vaiheelle annettiin selvät reunaehdot (kieli, kirjastoversiot, testivaatimus, tietoturvavaatimukset),
ja jokainen vaihe verifioitiin testeillä ja oikeaa palvelinta vasten ennen committia.

### Mitä tekoäly teki hyvin

- Rakensi laajan kokonaisuuden johdonmukaisesti: sama virhemuoto, samat konventiot kaikissa
  reiteissä, testit joka vaiheeseen.
- Tarkisti kirjastojen ajantasaiset API:t dokumentaatiosta sen sijaan, että olisi arvannut. Tämä
  oli välttämätöntä, koska useat kirjastot olivat muuttuneet: argon2 0.6 generoi suolan itse eikä
  tunne `std`-featurea, jsonwebtoken 10+ vaatii kryptotaustan valinnan, reqwest 0.13 vaatii `form`-
  ja `query`-featuret erikseen, aes-gcm 0.11 vaihtoi `from_slice`-kutsut `TryFrom`-muotoon.
- Löysi ja korjasi omat virheensä käännösvirheiden perusteella nopeasti.

### Missä tekoälyn tuotosta piti arvioida kriittisesti tai se erehtyi

| Tilanne | Mitä tapahtui | Miten havaittiin ja korjattiin |
|---|---|---|
| Sync-moottorin sulkeumat | Ensimmäinen toteutus ei kääntynyt (`Send is not general enough`) | Kääntäjä; korjattiin trait-pohjaisella ratkaisulla (ADR-8) |
| Postgresin `avg()` | Viikkonäkymän keskiarvot olivat `numeric`, jota sqlx ei tue ilman lisäfeaturea | Käännösvirhe; lisättiin migraatio, joka castaa `double precision`iksi |
| CI:n testivaihe | Kyselymakrot käännettiin tyhjää palvelukantaa vasten | CI kaatui; lisättiin `SQLX_OFFLINE=true` |
| CI:n compose-validointi | `env_file` vaati `.env`-tiedostoa, jota CI:ssä ei ole | CI kaatui; validointi kopioi esimerkkitiedoston. Kun api vaihdettiin nimettyyn muuttujalistaan (ei `env_file`), kopiota ei enää tarvita lainkaan |
| Logout-cookie | `CookieJar::remove` ei lähetä poistocookieta, jos pyynnössä ei ollut cookieta | Integraatiotesti kaatui; vaihdettiin `add` + `Max-Age=0` |
| Dev-portti 8080 | Windowsissa Hyper-V varaa portin | Bind-virhe; vaihdettiin 8787 |
| Polarin dev-redirect | Ehdotettu `localhost:8787` olisi pudottanut istuntocookien paluuohjauksessa | Huomattiin suunnittelussa; redirect Vite-proxyn kautta |
| Windowsin rivinvaihdot | Shell-skriptit olisivat tallentuneet CRLF-muodossa ja rikkoutuneet Linuxissa | Gitin varoitus; `.gitattributes` pakottaa LF:n |

Yhteinen havainto: tekoäly tuotti toimivan rungon nopeasti, mutta jokainen integraatiopiste
(kirjasto, CI, käyttöjärjestelmä, ulkoinen API) vaati todellisen ajon ja usein korjauksen.
Testit ja "aja se oikeasti" -periaate olivat tärkeämpiä kuin promptin laatu.

`[TÄYDENNÄ: oma kokemus, mitä opit tekoälyn ohjaamisesta, miten varmistit että ymmärrät koodin]`

## 6. Tietoturva

Ks. [ARKKITEHTUURI.md](ARKKITEHTUURI.md) luku 5. Tiivistetysti: salasanat argon2id, istunto
httpOnly-cookiessa SameSite=Lax, OAuth-`state` CSRF-suojana, Polar-token AES-256-GCM-salattuna,
kaikki SQL parametrisoitua, salaisuudet vain `.env`:ssä, ei avoimia portteja, julkisista
vastauksista poistettu tunnisteet.

## 7. Yhteenveto

`[TÄYDENNÄ: mitä opit, täyttyivätkö tavoitteet, jatkokehitys]`

Jatkokehitysideat: Polar-webhookit välittömään synkronointiin, harjoitusten sykenäytteet ja
reitit kirjautumisen taakse, muut datalähteet saman domainin alle.
