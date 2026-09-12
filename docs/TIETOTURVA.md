# Tietoturva: katselmointi ja korjaukset

Sovellukselle ja julkiselle sivustolle tehtiin tietoturvakatselmointi 11.9.2026: koodi (backend,
frontend, nginx, compose, CI), git-historia salaisuuksien varalta, riippuvuudet (`cargo audit`,
`npm audit`) sekä julkisen sivuston mustalaatikkotestaus (otsakkeet, reittien suojaus, CORS,
staattiset tiedostot, DNS). Sovelluskoodista ei löytynyt injektiota, autentikoinnin ohitusta,
salaisuuksien vuotoa eikä palvelimen IP-osoitteen paljastumista; löydökset koskivat pääosin
palvelun reunaa ja käyttöä.

Löydökset korjattiin 11.–12.9.2026. Tämä dokumentti on yksi luku löydöstä kohti: mikä oli
ongelma, mitä tehtiin, missä tiedostoissa ja miten korjaus todennettiin. Katselmoinnin
täysimittaista raporttia ei ole tässä repossa, koska repo on julkinen.

Ks. myös [ARKKITEHTUURI.md](ARKKITEHTUURI.md) luku 5 (tietoturvaratkaisut kootusti) ja
[JULKAISU.md](JULKAISU.md) (julkaisu ja Cloudflare-asetukset).

---

## 1. Kirjautumisen suojaus

**Ongelma.** `POST /api/auth/login` käynnisti argon2id-laskennan jokaiselle pyynnölle ilman mitään
rajaa, eikä kirjautumisella ollut rate limitiä missään kerroksessa. Yksi vertailu varaa 19 MiB
muistia koko kestonsa ajaksi, ja koska laskenta ajetaan `spawn_blocking`illa tokion blokkaavassa
säiepoolissa (oletuksena 512 paikkaa), rajaton kirjautumistulva olisi yrittänyt varata luokkaa
9 GiB muistia. Kontti olisi kaatunut muistin loppumiseen ennen kuin yksikään kirjautuminen
valmistuu. CPU oli toissijainen ongelma: release-buildissa yksi vertailu maksaa noin 10–16 ms.

**Korjaus, kerros 1: IP-kohtainen raja nginxissä.** Kirjautumiset on rajattu 10 pyyntöön
minuutissa per IP (`limit_req`, burst 5). Raja kohdistetaan `map`-muuttujalla eikä omalla
location-lohkolla: tyhjällä avaimella nginx ei laske pyyntöä lainkaan, joten muut `/api/`-pyynnöt
eivät kuluta kirjautumisen kvoottia eikä proxy-asetuksia tarvitse monistaa. Avain luetaan
`$uri`:sta, joka on normalisoitu ja purettu, joten `/api/%61uth/login` ei ohita rajaa. Lisäksi koko
rajapinnalla on väljä katto (30 r/s, burst 60) halpojen reittien tulvitusta vastaan.

**Korjaus, kerros 2: rinnakkaisuusraja sovelluksessa.** `AppState` sai `Semaphore`n, jossa on
`LOGIN_MAX_CONCURRENT` (2) permittiä. Permit varataan heti syötteen tarkistuksen jälkeen mutta
ennen kantakyselyä, joten hylätty pyyntö ei varaa yhteyttä poolista eikä käynnistä laskentaa.
Ilman permittiä vastaus on heti `429` ja `Retry-After: 1`. Tämä kerros tarvitaan yhä, koska
IP-kohtainen raja ei auta hajautetussa tulvassa: se on viimeinen suoja muistille ja CPU:lle.

Olennainen yksityiskohta: permit **siirretään `spawn_blocking`-sulkeumaan** eikä jätetä elämään
pyynnön futureen. Jos asiakas katkaisee yhteyden, axum pudottaa futuren, mutta blokkaavaa tehtävää
ei voi keskeyttää: argon2 jäisi ajamaan loppuun samalla kun vapautunut permit päästäisi jo
seuraavan laskennan käyntiin. Silloin rajan olisi voinut ohittaa kokonaan tulvalla, jossa yhteydet
katkaistaan heti.

| Tiedosto | Mitä muuttui |
|---|---|
| `frontend/nginx.conf` | `map` + kaksi `limit_req_zone`a, `limit_req_status 429`, rajat `/api/`-lohkossa |
| `backend/crates/api/src/auth/mod.rs` | vakiot `LOGIN_MAX_CONCURRENT` ja `LOGIN_RETRY_AFTER_SECS` perusteluineen |
| `backend/crates/api/src/state.rs` | kenttä `login_limit: Arc<Semaphore>` |
| `backend/crates/api/src/auth/password.rs` | `verify` ottaa permitin ja siirtää sen blokkaavaan tehtävään |
| `backend/crates/api/src/routes/auth.rs` | permitin varaus, 429-vastaus, 429 OpenAPI-kuvaukseen |
| `backend/crates/api/src/error.rs` | `TooManyRequests` sai oman viestikentän (ennen viesti oli kiinteä Polar-teksti) |
| `backend/crates/api/tests/common/mod.rs` | `test_app_and_state`, jotta testi pääsee käsiksi rajoittimeen |
| `backend/crates/api/tests/auth.rs` | integraatiotesti 429:lle, `Retry-After`-otsakkeelle ja permittien vapautumiselle |
| `frontend/src/pages/LoginPage.tsx` ja `LoginPage.test.tsx` | suomenkielinen viesti 429:lle |

**Todennus.** 20 samanaikaista kirjautumisyritystä samasta IP:stä tuotantopinoa vasten: 6 pääsi
läpi (401 väärästä salasanasta) ja 14 sai nginxin 429:n. nginxin virhelokissa lukee `client:
9.9.9.9`, eli raja laskee oikeaa asiakkaan IP:tä (`CF-Connecting-IP`) eikä tunnelin osoitetta;
heti perään toisesta IP:stä tullut pyyntö pääsi läpi. Ennen nginx-rajaa tehty mittaus osoitti
sovelluksen rajan toimivan: 40 samanaikaisesta kirjautumisesta 2 laski tiivisteen ja 38 hylättiin
429:llä noin 10 ms:ssa. Normaali käyttö ei törmää rajoihin: selaimella kaksi sivunlatausta
(16 pyyntöä) ja 10 peräkkäistä lukupyyntöä menivät kaikki läpi. Integraatiotesti hajoaa, jos
sovelluksen rajoitin ohitetaan — varmistettu mutatoimalla käsittelijä.

---

## 2. HTTPS-ohjaus

**Ongelma.** Sivusto vastasi myös `http://`-osoitteeseen täydellä sisällöllä eikä ohjannut
HTTPS:ään. Salaamattomalla yhteydellä reitin varrella oleva voi lukea liikenteen ja muokata sivua.
Origin ei voi päätellä asiakkaan protokollaa itse, koska se näkee tunnelista aina HTTP:n, ja
`nginx.conf` asettaa `X-Forwarded-Proto`n kiinteästi arvoon `https`.

**Korjaus.** Cloudflare kertoo asiakkaan protokollan `CF-Visitor`-otsakkeessa. nginx ohjaa
301:llä `https://`-osoitteeseen, jos siinä lukee `"scheme":"http"`. Yhdessä HSTS-otsakkeen
(luku 3) kanssa tämä sulkee aukon: ensimmäinen HTTPS-vastaus kertoo selaimelle, ettei tätä
nimeä haeta enää koskaan salaamattomana. Paikallisessa ajossa otsaketta ei ole, joten
`http://localhost:8088` toimii edelleen kehityksessä.

| Tiedosto | Mitä muuttui |
|---|---|
| `frontend/nginx.conf` | `CF-Visitor`-ehtoinen 301-ohjaus server-tasolla |
| `frontend/security-headers.conf` | HSTS (ks. luku 3) |

**Todennus.** Otsakkeella `CF-Visitor: {"scheme":"http"}` vastaus on `301` ja `Location` osoittaa
samaan polkuun ja kyselymerkkijonoon `https://`-muodossa. Otsakkeella `"scheme":"https"` ja ilman
otsaketta vastaus on `200`. Ohjaus koskee myös `/api/`-polkua.

**Reunan ohjaus.** Cloudflaren **Always Use HTTPS** kytkettiin päälle 12.9.2026 (SSL/TLS →
Edge Certificates), joten ohjaus tapahtuu jo reunalla eikä salaamaton pyyntö kulje tunnelin läpi
originille asti. Todennus: `http://biometrics.tonikiuru.com/api/health` vastaa 301:llä, jossa on
`Server: cloudflare` ja `CF-RAY` mutta ei yhtään turvaotsaketta. Otsakkeet tulevat vain originista,
joten niiden puuttuminen todistaa, ettei pyyntö käynyt nginxissä asti. Originin oma ohjaus jää
paikalleen varmistukseksi sen varalta, että reunan asetus joskus poistuu.

---

## 3. Selaimen turvaotsakkeet ja CSP

**Ongelma.** Sivusto ei palauttanut yhtään turvaotsaketta. Sivun sai upottaa kehykseen, mikä
mahdollistaa clickjackingin kirjautuneen omistajan napeille (esim. "Irrota Polar-tili"), eikä
mikään rajoittanut mistä sivu saa ladata skriptejä, jos jokin riippuvuus joskus vuotaisi XSS:n.

**Korjaus.** Otsakkeet ovat omassa tiedostossaan, joka sisällytetään server-tasolle ja erikseen
jokaiseen location-lohkoon. Erillinen include on pakollinen: nginx perii `add_header`-rivit vain
tasolle, joka ei aseta yhtään omaa, ja sekä `/assets/` että `/` asettavat Cache-Controlin. Ilman
tätä server-tason otsakkeet eivät olisi tulleet niihin lainkaan. Jokaisessa on `always`, joten
otsakkeet ovat myös virhevastauksissa (401, 404, 502).

Otsakkeet: HSTS, `X-Content-Type-Options`, `X-Frame-Options: DENY`, Referrer-Policy,
Permissions-Policy, Cross-Origin-Opener-Policy, Cross-Origin-Resource-Policy ja CSP. CSP:n tiukat
kohdat ovat `script-src 'self'` (inline-skriptejä ei ole missään eikä `unsafe-eval`-tarvetta),
`object-src 'none'`, `base-uri 'none'`, `frame-ancestors 'none'` ja `connect-src 'self'`.

| Tiedosto | Mitä muuttui |
|---|---|
| `frontend/security-headers.conf` | **uusi**: kaikki kahdeksan otsaketta perusteluineen |
| `frontend/nginx.conf` | include neljään kohtaan (server + kolme location-lohkoa), `server_tokens off` |
| `frontend/Dockerfile` | otsaketiedoston kopiointi imageen |
| `backend/crates/api/src/routes/mod.rs` | Swagger UI:n online-validator pois päältä (ks. luku 8) |

**Todennus.** Kaikilla poluilla on kaikki kahdeksan otsaketta: sovelluksen juuri, `/assets/`,
`/api/`-vastaus, Swagger UI ja 404. Sovellus ja kaaviot piirtyvät ilman yhtään CSP-rikkomusta
selaimen konsolissa. Vuoden mittainen välimuistitus ei vuoda 404-vastauksiin, koska
Cache-Controlissa ei ole `always`.

`style-src`in `'unsafe-inline'` tarvitaan vain Swagger UI:lle: React ja Recharts asettavat tyylit
DOM:n style-rajapinnan kautta, jota CSP ei koske. Tämä varmistettiin ajamalla sovellus arvolla
`style-src 'self'` — kaaviot piirtyivät oikein eikä rikkomuksia tullut.

---

## 4. Istuntojen mitätöinti

**Ongelma.** Istunto on tilaton JWT, joka on voimassa 7 vuorokautta. `CurrentUser` tarkisti vain
allekirjoituksen ja vanhenemisajan, joten palvelin ei voinut hylätä yhtäkään vielä voimassa olevaa
tokenia. Jos cookie vuotaisi, ainoa keino olisi vaihtaa `JWT_SECRET` ja käynnistää palvelu
uudelleen, mikä mitätöi kaikkien istunnot. Samasta syystä poistetun käyttäjän token olisi
toiminut vanhenemiseensa asti, ja rooli luettiin tokenista eikä kannasta.

**Korjaus.** `app_users` sai sarakkeen `token_version`, ja sama luku kulkee tokenin
`ver`-kentässä. Jokainen suojattu pyyntö hakee käyttäjän kannasta (yksi perusavainkysely) ja
vertaa lukua; samalla rooli ja sähköposti luetaan kannasta eikä tokenista. Version kasvattaminen
mitätöi kaikki käyttäjän istunnot:

```sql
UPDATE app_users SET token_version = token_version + 1 WHERE email = 'oma@osoite';
```

Uusi kirjautuminen toimii heti, koska se saa tokeniin uuden version. Poistetun käyttäjän token
lakkaa toimimasta välittömästi, samoin alennetun roolin oikeudet.

**Miksi luku eikä aikaleima.** Ensimmäinen toteutus vertasi tokenin `iat`-kenttää
`sessions_valid_from`-aikaleimaan. JWT:n `iat` on sekunnin tarkkuudella ja aikaleima
mikrosekunnin, joten kuluvan sekunnin sisällä vertailu oli epämääräinen: toteutus hylkäsi juuri
myönnetyn tokenin ja kaksi testiä kaatui. Luku on yksikäsitteinen eikä riipu kelloista.

**Hinta.** Yksi indeksoitu kysely per suojattu pyyntö. Julkiset lukureitit eivät tee sitä, jos
pyynnössä ei ole istuntocookieta, joten anonyymi kävijä ei aiheuta kyselyä.

| Tiedosto | Mitä muuttui |
|---|---|
| `backend/migrations/0006_session_invalidation.sql` | **uusi**: sarake `token_version` |
| `backend/crates/api/src/db/users.rs` | `token_version` kantariville, `invalidate_sessions` |
| `backend/crates/api/src/auth/jwt.rs` | `ver`-kenttä tokeniin, `issue` ottaa version |
| `backend/crates/api/src/auth/extract.rs` | käyttäjä haetaan kannasta, versio tarkistetaan |
| `backend/crates/api/src/routes/auth.rs` | kirjautuminen välittää version tokeniin |
| `backend/crates/api/tests/auth.rs` | testit mitätöinnille ja poistetulle käyttäjälle |

**Todennus.** Tuotantopinoa vasten: kirjautuminen 200, `/api/auth/me` 200, `token_version`
kasvatettiin kannassa, sama cookie 401, uusi kirjautuminen heti perään 200 ja uusi cookie 200.
Integraatiotestit kattavat saman sekä poistetun käyttäjän tokenin.

**Huom. julkaisusta.** Tokenin muoto muuttui: `Claims` vaatii nyt `ver`-kentän, jota ennen
julkaisua myönnetyissä tokeneissa ei ole. Niiden purku epäonnistuu, joten kaikki voimassa olevat
istunnot lakkaavat toimimasta heti kun uusi versio käynnistyy ja omistaja kirjautuu kertaalleen
uudelleen. Se on tarkoituksellista: migraation koko syy on se, ettei vanhoihin istuntoihin voinut
luottaa.

---

## 5. api-kontin ympäristömuuttujat

**Ongelma.** api-palvelulla oli `env_file: .env`, joten koko `deploy/.env` valui Rust-prosessin
ympäristöön — myös `CLOUDFLARE_TUNNEL_TOKEN`, `POSTGRES_PASSWORD`, `ADMIN_PASSWORD` ja `IMAGE_*`.
Tunnelin tokenilla api:n kaapannut hyökkääjä olisi voinut ajaa oman cloudflaredin ja ohjata
julkisen osoitteen omalle palvelimelleen.

**Korjaus.** `env_file` poistettiin ja tilalle tuli nimetty lista (allowlist) niistä 15
muuttujasta, jotka backend oikeasti lukee. Tunnelin token näkyy enää vain cloudflared-kontille.
Kantatunnukset api saa edelleen `DATABASE_URL`:ssa, jota se tarvitsee, mutta ei erillisinä
`POSTGRES_*`-muuttujina.

Koska `env_file` oli ainoa asia, joka vaati `.env`-tiedoston olemassaolon, CI:n
`cp deploy/.env.example deploy/.env` kävi tarpeettomaksi ja poistettiin. Tilalle tuli tarkistus,
joka kaatuu, jos backend alkaa lukea ympäristömuuttujaa, jota listalla ei ole. Ilman tarkistusta
uusi asetus jäisi tuotannossa hiljaa oletusarvoonsa ilman mitään virhettä.

| Tiedosto | Mitä muuttui |
|---|---|
| `deploy/docker-compose.yml` | api:n `env_file` → nimetty 15 muuttujan lista, ylläpito-ohje kommenttina |
| `.github/workflows/ci.yml` | turha `.env`-kopiointi pois, uusi vaihe joka tarkistaa listan kattavuuden |

**Todennus.** Ajossa olevan api-kontin ympäristössä on täsmälleen nämä 15 muuttujaa; `TUNNEL`,
`POSTGRES_` ja `IMAGE_` eivät esiinny siellä lainkaan. Tyhjästä kannasta käynnistys luo
omistajakäyttäjän normaalisti. Compose validoituu myös ilman `.env`-tiedostoa, eli CI:n
tilanteessa. Tarkistusvaihe kaatuu oikein, kun muuttuja poistetaan listalta.

---

## 6. Konttien oikeudet

**Ongelma.** Yksikään kontti ei rajoittanut Linux-kyvykkyyksiä eikä estänyt oikeuksien
korottamista setuid-binäärillä. Kontista karannut prosessi olisi saanut koko oletuskyvykkyysjoukon.

**Korjaus.** Kaikilla palveluilla on `no-new-privileges`. api ja cloudflared pudottavat kaikki
kyvykkyydet (`cap_drop: ALL`); api ajaa jo ei-root-käyttäjänä ja kuuntelee porttia 8080, joten se
ei tarvitse yhtään. web pudottaa kaikki paitsi neljä, jotka nginx tarvitsee käynnistyessään
(`CHOWN`, `SETGID`, `SETUID`, `NET_BIND_SERVICE`). db:n kyvykkyyksiä ei karsita, koska
postgres-image tarvitsee niitä datahakemiston omistajan vaihtoon.

| Tiedosto | Mitä muuttui |
|---|---|
| `deploy/docker-compose.yml` | `security_opt` kaikille, `cap_drop`/`cap_add` api:lle, web:lle ja cloudflaredille |

**Todennus.** `docker inspect` ajossa olevista konteista: api `CapDrop=[ALL]`, `CapAdd=[]`,
käyttäjä `app` (uid 10001); web `CapDrop=[ALL]` ja vain ne neljä lisättyä; molemmilla
`no-new-privileges:true`. nginxin työprosessit ajavat `nginx`-käyttäjänä (pääprosessi on rootina,
ks. alla). Koko pino käynnistyi terveeksi näillä rajoituksilla.

**Tietoisesti tekemättä.** Täysin ei-root nginx vaatisi `nginx-unprivileged`-imagen, joka
kuuntelee porttia 8080. Se tarkoittaisi muutosta myös Cloudflaren tunnelin public hostname
-kohteeseen (nyt `web:80`), eli sivusto olisi alhaalla kunnes asetus on päivitetty käsin. Sama
koskee kannan omaa ei-superuser-roolia: kaikki kyselyt ovat parametrisoituja käännösaikana
tarkistettuja makroja, joten se olisi vain syvyyssuuntaista suojaa, ja vaihto vaatisi
roolin luonnin ja omistajuuksien siirron olemassa olevassa kannassa.

---

## 7. CI ja toimitusketjun hygienia

**Ongelma.** Kolme asiaa, jotka kaikki koskevat sitä, mitä koodia putki ajaa ja millä oikeuksilla.
Työnkulun token sai repon oletusoikeudet, koska `permissions`-lohkoa ei ollut työnkulun tasolla.
Actionit oli kiinnitetty liikkuviin tageihin ja yksi haaraan (`@master`), joten actionin tekijän
tilin kaappaus tai yksi huono julkaisu olisi muuttanut sen, mitä CI ajaa, ilman yhtään muutosta
tässä repossa — ja CI:n token pääsee GHCR:ään eli julkaistuihin imageihin. Riippuvuuksien
haavoittuvuuksia ei tarkistettu missään.

**Korjaus.** Työnkulun tasolle `permissions: contents: read` (docker-työ nostaa itselleen
`packages: write`). Kaikki 12 action-viittausta kiinnitetty commitin SHA:han, versionumero
kommenttiin. Uusi `audit`-työ ajaa `cargo auditin` ja frontend-työ `npm auditin`. Tunnettu
rsa-löydös on ohitettu `backend/.cargo/audit.toml`-tiedostossa perusteluineen, eli ohitus on
näkyvä ja tarkistettavissa. `.github/dependabot.yml` avaa viikoittain päivitys-PR:t, jotta
SHA-kiinnitys ei jähmetä actioneita vanhoihin versioihin.

| Tiedosto | Mitä muuttui |
|---|---|
| `.github/workflows/ci.yml` | oletusoikeudet, 12 actionia SHA:han, `audit`-työ, `npm audit`, varmistus ympäristölistan tarkistukseen |
| `.github/dependabot.yml` | **uusi**: viikoittaiset päivitykset actioneille, Cargolle, npm:lle ja pohjaimageille |
| `backend/.cargo/audit.toml` | **uusi**: tietoisesti ohitettu RUSTSEC-2023-0071 perusteluineen |

**Todennus.** `cargo audit` ja `npm audit --omit=dev --audit-level=high` päättyvät koodiin 0
samoilla komennoilla kuin CI:ssä. Työnkulku läpäisee `actionlintin` ilman huomautuksia, ja kaikki
12 viittausta on 40 merkin SHA. Ympäristölistan tarkistus testattiin neljällä tapauksella:
nykyinen repo läpäisee, ja tarkistus kaatuu oikein vanhentuneesta hakulausekkeesta,
lukukelvottomasta compose-lohkosta ja listalta puuttuvasta muuttujasta.

---

## 8. Pienemmät korjaukset

| Asia | Ongelma | Korjaus | Tiedosto |
|---|---|---|---|
| nginxin yhteydet backendiin | nginx avasi uuden TCP-yhteyden joka pyynnölle; kovassa tulvassa kontin efemeeriset portit loppuivat ja nginx palautti 502 vaikka backend vastasi | `upstream`-lohko ja `keepalive 32`. Mitattu 12 s tulvalla: ennen 9971 kpl 502-vastauksia, jälkeen 0 | `frontend/nginx.conf` |
| Lokien koko | Dockerin oletusajuri kasvattaa json-lokia rajatta samalla levyllä, jolla kannan data on | `max-size: 10m`, `max-file: 3` api- ja web-konteille | `deploy/docker-compose.yml` |
| Lokin eheys | kirjautumisvirheen lokirivi kirjoitti käyttäjän syöttämän sähköpostin sellaisenaan, joten siihen pystyi upottamaan rivinvaihdoilla omia lokirivejä | `%email` → `?email`, joka escapettaa ohjausmerkit | `backend/crates/api/src/routes/auth.rs` |
| Sivutuksen ylivuoto | `(page - 1) * per_page` laskettiin `u32`:na: suuri `page` kiersi ympäri (release) tai panikoi 500:ksi (debug) | laskenta `i64`:nä | `backend/crates/api/src/routes/data/records.rs` |
| Varmuuskopiot | `pg_dump`-tiedostot syntyivät oletusoikeuksilla kotihakemistoon | `umask 077` ja `chmod 600`; kommentti muistuttaa kopion siirtämisestä koneen ulkopuolelle | `deploy/backup.sh` |
| Swagger UI:n validator | `/api/docs/` latasi merkkikuvan validator.swagger.io:sta ja vuoti API-kuvauksen osoitteen kolmannelle osapuolelle | `validator_url("none")` | `backend/crates/api/src/routes/mod.rs` |
| 429:n viesti käyttäjälle | backend palauttaa englanninkielisen viestin API-kuluttajille, ja käyttöliittymä näyttäisi sen sellaisenaan suomenkielisellä sivulla | oma suomenkielinen teksti ja testi | `frontend/src/pages/LoginPage.tsx` |

---

## 9. Katselmoinnin tila

| Löydös | Tila |
|---|---|
| 1. Selain sai olla yhteydessä ilman HTTPS-ohjausta | **korjattu** kahdessa kerroksessa (luku 2): origin ohjaa itse ja Cloudflaren Always Use HTTPS on päällä |
| 2. Kirjautumisella ei ollut mitään rajoitusta | **korjattu** kahdessa kerroksessa (luku 1) |
| 3. Istuntoa ei voi mitätöidä palvelimelta | **korjattu** (luku 4) |
| 4. Ei turvaotsakkeita | **korjattu** (luku 3) |
| 5. Tunnelin token api-prosessin ympäristössä | **korjattu** (luku 5) |
| 6. Julkinen näyteikkuna näyttää tarkkaa dataa | **tiedostettu valinta**; paino ja pituus piilotettu kirjautumattomilta (`PUBLIC_BODY_METRICS`) |
| 7. `rsa`-kirjaston aikakanava-haavoittuvuus | **ohitettu tietoisesti**: riippuvuus tulee `jsonwebtoken`in kautta, mutta sovellus allekirjoittaa vain HS256:lla eikä RSA-koodia ajeta. Ohitus ja perustelu `backend/.cargo/audit.toml`, ja `cargo audit` ajaa CI:ssä |
| 8. CI:n ja toimitusketjun hygienia | **korjattu** (luku 7) |
| 9. Konttien ja kannan oikeudet | **osin korjattu** (luku 6): kyvykkyydet karsittu, mutta nginxin pääprosessi on yhä root ja sovellus käyttää kannan superuseria |
| 10. Lokiin kirjoitettiin käyttäjän syöte sellaisenaan | **korjattu** (luku 8) |
| 11. Sivutuksen ylivuoto | **korjattu** (luku 8) |
| 12. Swagger UI ja versiotieto julkisia | **tiedostettu valinta**; kolmannen osapuolen validator-kutsu poistettu |
| 13. Varmuuskopiot salaamattomina kotihakemistossa | **osin korjattu** (luku 8): oikeudet kunnossa, kopio koneen ulkopuolelle on yhä tekemättä |

---

## 10. Seuraavat askeleet

Cloudflaren hallintapaneelissa, ei koodimuutoksia:

1. Valinnainen: **rate limiting -sääntö** `/api/auth/login`-polulle. nginx rajaa jo per IP, mutta
   reunalla tulva ei kuluta edes tunnelin kapasiteettia.

Muuta, kun aika riittää:

2. Varmuuskopio myös koneen ulkopuolelle (luku 9, löydös 13).
3. Kannan oma ei-superuser-rooli ja `nginx-unprivileged` (luku 6) — molemmat vaativat
   käsityötä julkaisussa, joten niitä ei tehty ohessa.
