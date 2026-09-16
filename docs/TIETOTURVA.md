# Tietoturva: katselmointi ja korjaukset

Sovellukselle ja julkiselle sivustolle tehtiin tietoturvakatselmointi 11.9.2026: koodi (backend,
frontend, nginx, compose, CI), git-historia salaisuuksien varalta, riippuvuudet (`cargo audit`,
`npm audit`) sekä julkisen sivuston mustalaatikkotestaus (otsakkeet, reittien suojaus, CORS,
staattiset tiedostot, DNS). Sovelluskoodista ei löytynyt injektiota, autentikoinnin ohitusta,
salaisuuksien vuotoa eikä palvelimen IP-osoitteen paljastumista; löydökset koskivat pääosin
palvelun reunaa ja käyttöä.

Löydökset korjattiin 11.–12.9.2026. Jälkikatselmointi 16.9.2026 kävi koodin, julkaisun ja
git-historian uudelleen läpi: yksi uusi löydös (päivämäärävälin alivuoto, luku 8), muut kohdat
ennallaan. Tämä dokumentti on yksi luku löydöstä kohti: mikä oli ongelma, mitä tehtiin, missä
tiedostoissa ja miten korjaus todennettiin. Katselmoinnin täysimittaista raporttia ei ole tässä
repossa, koska repo on julkinen.

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

## 6. Konttien ja kannan oikeudet

**Ongelma.** Yksikään kontti ei rajoittanut Linux-kyvykkyyksiä eikä estänyt oikeuksien
korottamista setuid-binäärillä. Kontista karannut prosessi olisi saanut koko oletuskyvykkyysjoukon.

**Korjaus.** Kaikilla palveluilla on `no-new-privileges`. api, web ja cloudflared pudottavat
kaikki kyvykkyydet (`cap_drop: ALL`) eivätkä ota yhtään takaisin: api ajaa ei-root-käyttäjänä
(uid 10001) ja kuuntelee porttia 8080, ja web vaihtui nginx-unprivileged-imageen (ks. alla). db:n
kyvykkyyksiä ei karsita, koska postgres-image tarvitsee niitä datahakemiston omistajan vaihtoon.

| Tiedosto | Mitä muuttui |
|---|---|
| `deploy/docker-compose.yml` | `security_opt` kaikille ja `cap_drop: ALL` api:lle, web:lle ja cloudflaredille |

**Todennus.** `docker inspect` ajossa olevista konteista: api ja web molemmat `CapDrop=[ALL]`,
`CapAdd=[]` ja `no-new-privileges:true`; api:n käyttäjä on `app` (uid 10001) ja web:n `nginx`
(uid 101). Kumpikaan ei aja yhtään prosessia rootina. Koko pino käynnistyi terveeksi näillä
rajoituksilla.

### Kannan sovellusrooli

**Ongelma.** Sovellus yhdisti kantaan `POSTGRES_USER`-tunnuksella, jonka postgres-image luo
**superuserina**. Kaikki kyselyt ovat parametrisoituja, käännösaikana tarkistettuja sqlx-makroja,
joten SQL-injektiota ei ole — mutta superuser tarkoittaa, että jos api-prosessi joskus kaapataan,
hyökkääjä saa kannan kautta käyttöjärjestelmäkomentoja (`COPY ... FROM PROGRAM`), voi lukea ja
kirjoittaa palvelimen tiedostoja, luoda rooleja ja ohittaa rivitason suojaukset.

**Korjaus.** Sovellukselle luodaan oma rooli ilman superuser-oikeuksia. Se omistaa public-skeeman,
joten se voi ajaa migraatiot, mutta ei mitään muuta. `POSTGRES_USER` jää ylläpitoon: `pg_dump`,
`psql` ja kannan omistajuus.

Rooli otetaan käyttöön kahdella rivillä `deploy/.env`-tiedostossa (`DB_APP_USER`,
`DB_APP_PASSWORD`). Jos ne ovat tyhjiä, api käyttää superuseria kuten ennen — pelkkä uuden
compose-tiedoston käyttöönotto ei siis muuta mitään, ja paluu entiseen on kahden rivin
kommentointi. `deploy.sh` kaatuu, jos vain toinen on asetettu, koska silloin `DATABASE_URL` saisi
väärän käyttäjä/salasana-yhdistelmän.

Uusi asennus saa roolin itsestään (`deploy/initdb/10-app-role.sh`, jonka postgres-image ajaa vain
tyhjään datahakemistoon). Olemassa oleva kanta siirretään kertaluonteisesti:
`deploy/sql/app-role-handover.sql`.

| Tiedosto | Mitä muuttui |
|---|---|
| `deploy/initdb/10-app-role.sh` | **uusi**: luo roolin ensimmäisellä käynnistyksellä |
| `deploy/sql/app-role-handover.sql` | **uusi**: kertaluonteinen siirto olemassa olevalle kannalle |
| `deploy/docker-compose.yml` | init-skriptin liitos, roolin muuttujat db:lle, `DATABASE_URL` käyttää roolia jos se on asetettu |
| `deploy/deploy.sh` | tarkistus: molemmat muuttujat tai ei kumpaakaan |
| `deploy/.env.example` | uudet muuttujat ja perustelut |

**Todennus.** Koko tuotantopino ajettiin paikallisesti roolin kanssa: init-skripti loi roolin, api
yhdisti sillä (`pg_stat_activity` näyttää kaksi `polar_app`-yhteyttä) ja ajoi **itse kaikki kuusi
migraatiota** — myös `CREATE EXTENSION pgcrypto`, koska pgcrypto on PostgreSQL 18:ssa
"trusted"-laajennus, jonka saa asentaa `CREATE`-oikeudella ilman superuseria. Skeema ja kaikki
kymmenen taulua ovat roolin omistuksessa, kanta itse superuserin. Kirjautuminen, istunnon
mitätöinti ja datan luku toimivat. Kahdeksan vaarallista komentoa estyy: `COPY ... FROM PROGRAM`,
`pg_read_file`, `CREATE ROLE`, `ALTER ROLE ... SUPERUSER`, `pg_authid`-lukeminen, ei-trusted
laajennuksen asennus, `ALTER SYSTEM` ja `DROP DATABASE`.

Siirto olemassa olevaan kantaan todennettiin erikseen simuloimalla tuotanto (superuser omistaa
kaiken, `_sqlx_migrations` ja dataa paikallaan): siirron jälkeen rooli tekee sekä DML:ää että
DDL:ää, ja `pg_dump`/palautus säilyttää omistajuudet. Toinen ajo kaatuu siististi virheeseen
"role already exists" eikä jätä puolittaista tilaa, koska koko siirto on yhdessä transaktiossa.

**Sudenkuoppa, joka löytyi kokeilemalla.** `REASSIGN OWNED BY <superuser> TO ...` ei toimi:
PostgreSQL hylkää sen virheellä *"cannot reassign ownership of objects owned by role ... because
they are required by the database system"*, koska bootstrap-superuser omistaa myös
järjestelmäobjekteja. Siksi omistajuus siirretään objekti kerrallaan, ja pgcrypton omat 37
funktiota jätetään koskematta (`pg_depend.deptype = 'e'`).

**Mitä tämä ei suojaa.** Rooli omistaa taulut, joten se voi yhä pudottaa ja muuttaa niitä — se on
migraatioiden hinta. Suoja kohdistuu nimenomaan SQL:stä käyttöjärjestelmään johtavaan tiehen ja
muiden roolien koskemiseen.

### Täysin ei-root nginx

**Ongelma.** nginxin pääprosessi ajoi rootina. Se on nginxin normaali toimintatapa — pääprosessi
varaa portin 80 ja pudottaa työprosessit `nginx`-käyttäjälle — mutta se tarkoittaa, että kontissa
on jatkuvasti root-prosessi, joka käsittelee verkosta tulevaa liikennettä. Sitä varten kontti
tarvitsi neljä kyvykkyyttä: `CHOWN`, `SETGID` ja `SETUID` käyttäjän vaihtoon ja `NET_BIND_SERVICE`
portin varaamiseen.

**Korjaus.** Pohjaimage vaihdettiin `nginxinc/nginx-unprivileged:alpine`-imageen, jossa myös
pääprosessi ajaa uid 101:llä. Kaikki neljä kyvykkyyttä poistettiin: web ajaa nyt `cap_drop: ALL`
ilman yhtään `cap_add`-riviä.

Portti pysyy 80:ssä, vaikka image kuuntelee oletuksena 8080:aa. Näin Cloudflaren tunnelin public
hostname -kohde (`web:80`) pysyy ennallaan eikä sivusto käy alhaalla käyttöönoton takia.
Ei-root-prosessi saa varata alle 1024:n portin vain, jos kontin verkkonimiavaruudessa
`net.ipv4.ip_unprivileged_port_start` on nolla, ja se asetetaan compose-tiedostossa. Asetus koskee
vain tämän yhden kontin omaa verkkonimiavaruutta, jossa ei aja mitään muuta kuin nginx, joten se ei
anna hyökkääjälle uutta kykyä: portin varaaminen omassa nimiavaruudessa ei johda mihinkään.

| Tiedosto | Mitä muuttui |
|---|---|
| `frontend/Dockerfile` | pohjaimage `nginx:alpine` → `nginxinc/nginx-unprivileged:alpine` |
| `deploy/docker-compose.yml` | `cap_add`-lohko pois web:ltä, tilalle `sysctls: net.ipv4.ip_unprivileged_port_start: "0"` |

**Todennus.** Oikea `nginx.conf` ajettiin unprivileged-imagessa kolmella eri
asetusyhdistelmällä, ja lopuksi koko tuotantopino paikallisesti:

1. Tavoitetila (sysctl 0, nolla kyvykkyyttä): `ps` näyttää pääprosessin ja kaikki 32 työprosessia
   `nginx`-käyttäjänä (uid 101), **nolla root-prosessia**. `docker inspect`: `CapDrop=[ALL]`,
   `CapAdd=[]`. nginx kuuntelee `0.0.0.0:80`.
2. Tavallisen Linux-palvelimen oletus jäljiteltynä (`ip_unprivileged_port_start=1024`): nginx
   kaatuu heti, `bind() to 0.0.0.0:80 failed (13: Permission denied)`.
3. Sama, mutta `NET_BIND_SERVICE` lisättynä: **kaatuu silti samaan virheeseen**. Kyvykkyys ei
   välity ei-root-prosessille, koska Docker ei aseta ambient-kyvykkyyksiä. Siksi oikea keino on
   sysctl eikä `cap_add`.
4. Koko pino paikallisesti: web on `healthy`, ja etusivu, SPA-fallback, `/assets/`:n
   välimuistiotsake, kirjautuminen ja istunto toimivat. Turvaotsakkeita tulee 8/8 kaikista
   kolmesta location-lohkosta, ja kirjautumisen pyyntöraja toimii ennallaan (6 × 401, sitten
   12 × 429). 1,45 MB:n välitetty vastaus tuli tavu tavulta oikein myös hidastetulle asiakkaalle
   eikä nginxin lokiin tullut yhtään virhettä: nginx loi itse uid 101:llä kaikki viisi
   väliaikaishakemistoaan `/tmp`:hen, jonne tämä image ohjaa ne.
5. **Tunnelin polku erikseen:** toinen kontti samassa compose-verkossa saa osoitteista
   `http://web:80/` ja `http://web:80/api/health` vastauksen HTTP 200 — eli juuri sen, mitä
   cloudflared tekee. Cloudflaren asetuksiin ei siis tarvitse koskea.

**Sudenkuoppa, joka löytyi kokeilemalla.** Docker Desktop (WSL2) laskee
`ip_unprivileged_port_start`-rajan nollaan valmiiksi, tavallinen Linux-palvelin ei. Ilman
nimenomaista sysctl-riviä pino olisi siis toiminut kehityskoneella ja kaatunut palvelimella — juuri
se tapaus, jota paikallinen koeajo ei itsestään paljasta.

**Yhteensopivuus vanhaan imageen.** Mitattu molempiin suuntiin. Uusi image vanhalla
compose-asetuksella (ylimääräiset `cap_add`-rivit mukana) toimii normaalisti eikä aja mitään
rootina, eli käyttöönoton järjestys ei haittaa. Toisin päin ei: jos `IMAGE_TAG` kiinnitetään
vanhaan, root-pohjaiseen web-imageen uuden compose-tiedoston kanssa, kontti kaatuu silmukkaan
virheeseen `chown("/var/cache/nginx/client_temp", 101) failed (1: Operation not permitted)`. Image
ja compose-tiedosto liikkuvat siis yhdessä (ks. docs/JULKAISU.md luku 7).

**Tietoisesti tekemättä.** Erillistä migraatioroolia kantaan ei tehty: sovellus ajaa migraatiot
itse käynnistyessään, joten se tarvitsisi kaksi yhteysosoitetta.

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
| Päivämäärävälin alivuoto | `RangeQuery::resolve` laski 30 päivän oletusalun `to - 29 päivää` tarkistamattomalla vähennyslaskulla, ja `unwrap_or` laski sen aina, myös kun `from` oli annettu. Kyselyparametri `to=-262143-01-01` (NaiveDaten alaraja) panikoi käsittelijän ilman kirjautumista: yhteys katkesi, nginx vastasi 502 ja pino jäi api:n lokiin. Koski reittejä `/api/sleep`, `/recharge`, `/activity`, `/cardio-load` ja `/summary/daily`; palvelin itse pysyi pystyssä | `checked_sub_signed`, joka palauttaa 400:n, ja oletusalku lasketaan vain kun `from` puuttuu. Regressiotestit: yksikkötesti `resolve`lle ja integraatiotesti kaikille viidelle reitille, molemmat todennettu kaatuvan vanhalla koodilla. Kelvolliset mutta äärimmäiset päivämäärät (ennen vuotta 4713 eaa., PostgreSQL:n alaraja) menevät yhä kantaan asti ja antavat 500:n eikä 400:aa; ne eivät kaada mitään. Löydetty jälkikatselmoinnissa 16.9.2026 | `backend/crates/api/src/routes/data/mod.rs`, `backend/crates/api/tests/data.rs` |
| Varmuuskopiot | `pg_dump`-tiedostot syntyivät oletusoikeuksilla kotihakemistoon | `umask 077` ja `chmod 600`; etäkopio luvussa 9 | `deploy/backup.sh` |
| Swagger UI:n validator | `/api/docs/` latasi merkkikuvan validator.swagger.io:sta ja vuoti API-kuvauksen osoitteen kolmannelle osapuolelle | `validator_url("none")` | `backend/crates/api/src/routes/mod.rs` |
| 429:n viesti käyttäjälle | backend palauttaa englanninkielisen viestin API-kuluttajille, ja käyttöliittymä näyttäisi sen sellaisenaan suomenkielisellä sivulla | oma suomenkielinen teksti ja testi | `frontend/src/pages/LoginPage.tsx` |

---

## 9. Etävarmuuskopio

**Ongelma.** Nimenomaan se kopio, jonka pitäisi pelastaa tilanne, oli samalla levyllä kuin kanta:
levyrikko, varastettu kone, kiristyshaittaohjelma tai väärin kirjoitettu `docker compose down -v`
olisi vienyt datan ja varmuuskopiot samalla kertaa.

**Korjaus.** `deploy/backup.sh` lähettää päivittäisen dumpin myös koneen ulkopuolelle. Kopio
salataan ennen lähtöä `age`lla **julkisella avaimella**, joten palvelin voi kirjoittaa
varmuuskopioita mutta ei lukea omia vanhoja kopioitaan. Yksityinen avain ei ole palvelimella
lainkaan.

Kohde on mikä tahansa `rclone`n tukema paikka (esimerkit `.env.example`:ssa: Cloudflare R2 ja
SFTP). Tunnukset annetaan `RCLONE_CONFIG_*`-ympäristömuuttujina, jotka tulevat samasta
`deploy/.env`-tiedostosta kuin muut salaisuudet, joten erillistä `rclone.conf`-tiedostoa ja toista
suojattavaa tiedostoa ei tarvita. Ilman asetuksia skripti toimii kuten ennen ja varoittaa, ettei
etäkopiota ole.

Paikallinen kopio jää tarkoituksella salaamattomaksi: se on samalla koneella kuin kanta, joten
salaus ei suojaisi miltään uudelta, ja se hankaloittaisi rutiinipalautusta. Oikeudet ovat
`umask 077` + `chmod 600`.

| Tiedosto | Mitä muuttui |
|---|---|
| `deploy/backup.sh` | koon järkevyystarkistus, salaus, lähetys, koon varmistus, etäsäilytys |
| `deploy/.env.example` | `BACKUP_AGE_RECIPIENT`, `BACKUP_REMOTE`, `BACKUP_REMOTE_KEEP_DAYS` ja esimerkit |

**Todennus.** Oikea `backup.sh` ajettiin Debian-kontissa (sama käyttöjärjestelmä kuin
palvelimella), `docker`-komento tuettuna ja paikallinen hakemisto rclone-kohteena. Viisi tapausta:

1. Onnistunut ajo: paikallinen kopio oikeuksin `600`, salattu kopio perillä, ja purku yksityisellä
   avaimella antaa **tavu tavulta saman** tiedoston takaisin.
2. Epäonnistunut `pg_dump` (20 tavua): ajo keskeytyy koodilla 1 **eikä lähetä mitään**. Ilman tätä
   rikkinäinen kopio olisi korvannut toimivat etäkopiot päivä kerrallaan — tyhjä tiedosto
   salautuu ja latautuu aivan yhtä hyvin kuin kunnollinen. Raja 1000 tavua on mitattu: pelkkä
   skeema pakkautuu 3567 tavuun, epäonnistunut dump 20:een.
3. Tavoittamaton kohde: virhe näkyy, paluukoodi on 1 ja **paikallinen kopio jää silti talteen**.
   rclonen uudelleenyritykset on rajattu, jotta cron-ajo ei jumitu minuuteiksi.
4. Asetukset puuttuvat: paikallinen kopio onnistuu, varoitus lokiin, paluukoodi 0.
5. Säilytys: yli 90 päivää vanha etäkopio poistuu, tuore jää.

**Avaimen menettäminen.** Jos yksityinen age-avain katoaa, etäkopiot ovat lopullisesti auki
saamatta. Avain luodaan omalla koneella, ei palvelimella, ja talletetaan salasanojen hallintaan.
Paikalliset kopiot ovat salaamattomia, joten ne toimivat varareittinä.

---

## 10. Katselmoinnin tila

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
| 9. Konttien ja kannan oikeudet | **korjattu** (luku 6): yksikään kontti ei aja prosessia rootina, kaikki kyvykkyydet on pudotettu ja sovelluksella on kannassa oma ei-superuser-rooli |
| 10. Lokiin kirjoitettiin käyttäjän syöte sellaisenaan | **korjattu** (luku 8) |
| 11. Sivutuksen ylivuoto | **korjattu** (luku 8) |
| 12. Swagger UI ja versiotieto julkisia | **tiedostettu valinta**; kolmannen osapuolen validator-kutsu poistettu |
| 13. Varmuuskopiot salaamattomina kotihakemistossa | **korjattu**: oikeudet kunnossa (luku 8) ja salattu kopio koneen ulkopuolelle (luku 9) |
| 14. Päivämäärävälin alivuoto panikoi pyynnön | **korjattu** (luku 8) regressiotesteineen; löydetty jälkikatselmoinnissa 16.9.2026 |

---

## 11. Seuraavat askeleet

Cloudflaren hallintapaneelissa, ei koodimuutoksia:

1. Valinnainen: **rate limiting -sääntö** `/api/auth/login`-polulle. nginx rajaa jo per IP, mutta
   reunalla tulva ei kuluta edes tunnelin kapasiteettia.

Palvelimella, kertaluonteisesti (ks. docs/JULKAISU.md luku 6b):

2. Ota etävarmuuskopio käyttöön: `apt-get install age rclone`, luo avainpari omalla koneella, luo
   kohde ja täytä kolme muuttujaa `deploy/.env`-tiedostoon. Testaa palautus heti — varmuuskopio,
   jota ei ole kerran palautettu, on arvaus.
3. Ota kannan sovellusrooli käyttöön: aseta `DB_APP_USER` ja `DB_APP_PASSWORD` ja aja
   `deploy/sql/app-role-handover.sql` kertaluonteisesti olemassa olevalle kannalle.

Koodin puolelta ei jää avoimia kohtia: kaikki 14 löydöstä (13 katselmoinnista 11.9.2026 ja yksi
jälkikatselmoinnista 16.9.2026) on korjattu tai kirjattu tiedostetuksi valinnaksi (luku 10).
Kohdat 2 ja 3 yllä ovat pelkkää käyttöönottoa — molemmat muutokset ovat valmiina repossa ja
odottavat vain rivejä `deploy/.env`-tiedostossa.
