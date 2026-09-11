# Julkaisuohje: biometrics.tonikiuru.com

Kohde: kotipalvelimen Proxmox-kontti `pve2`. Sovellus ajetaan Docker Composella
ja julkaistaan internetiin Cloudflare Tunnelilla. Palvelimelle ei avata yhtään porttia, TLS
päätetään Cloudflaressa ja tunneli on salattu.

```
Selain ──HTTPS──> Cloudflare ──tunneli──> cloudflared ──> nginx (web) ──/api──> api ──> db
```

Ohje on kirjoitettu Debian 12/13 -pohjaiselle LXC-kontille. Ubuntu toimii samoin.

## 0. Tarkistuslista ennen aloitusta

- [ ] Proxmox-kontissa on **nesting** päällä (Docker ei muuten käynnisty LXC:ssä):
      Proxmox → kontti → Options → Features → `nesting=1`. Unprivileged-kontti riittää.
- [ ] Kontilla on vähintään 1 CPU, 1 GB RAM ja 8 GB levyä. Imaget rakennetaan GitHub Actionsissa,
      joten palvelin ei käännä Rustia. (Jos haluat rakentaa palvelimella, varaa 4 GB RAM.)
- [ ] GitHub-repo on julkinen **tai** palvelimelle on tehty `docker login ghcr.io` (kohta 4).
- [ ] `tonikiuru.com` on Cloudflaren nimipalvelimilla (on).
- [ ] Polar-kehittäjätilillä (https://admin.polaraccesslink.com) on **tuotantoasiakas**, jonka
      redirect URL on täsmälleen `https://biometrics.tonikiuru.com/api/polar/callback`.
      Dev-asiakasta (`http://localhost:5173/...`) ei voi käyttää, koska redirect URL on kiinteä.

## 1. Docker palvelimelle

Kirjaudu konttiin (`pct enter <id>` Proxmoxin konsolista tai `ssh root@<pve2:n LAN-osoite>`).

```bash
apt-get update && apt-get install -y ca-certificates curl git
install -m 0755 -d /etc/apt/keyrings
curl -fsSL https://download.docker.com/linux/debian/gpg -o /etc/apt/keyrings/docker.asc
echo "deb [arch=$(dpkg --print-architecture) signed-by=/etc/apt/keyrings/docker.asc] \
  https://download.docker.com/linux/debian $(. /etc/os-release && echo "$VERSION_CODENAME") stable" \
  > /etc/apt/sources.list.d/docker.list
apt-get update
apt-get install -y docker-ce docker-ce-cli containerd.io docker-compose-plugin
systemctl enable --now docker
docker run --rm hello-world     # pitää tulostaa "Hello from Docker!"
```

Jos `hello-world` epäonnistuu virheellä `permission denied` tai `overlayfs`, nesting ei ole päällä
(kohta 0) tai kontti pitää käynnistää uudelleen asetuksen muuttamisen jälkeen.

Suositus: aja sovellus omalla käyttäjällä, ei rootilla.

```bash
useradd -m -s /bin/bash -G docker polar
su - polar
```

## 2. Koodi ja asetukset

```bash
git clone https://github.com/Tonzium/biometrics.git polar-data-hub
cd polar-data-hub
cp deploy/.env.example deploy/.env
chmod 600 deploy/.env
nano deploy/.env
```

Täytä `deploy/.env` seuraavasti. Salaisuudet generoidaan komennoilla, älä keksi niitä itse.

| Muuttuja | Arvo tuotannossa |
|---|---|
| `POSTGRES_PASSWORD` | `openssl rand -hex 24` (**vain kirjaimia ja numeroita**: salasana upotetaan yhteysosoitteeseen `postgres://polar:<salasana>@db`, ja base64:n `/`, `+` ja `=` rikkovat sen) |
| `DATABASE_URL` | saa jäädä esimerkkiarvoon; compose ylikirjoittaa sen (`db`-kontti) |
| `BIND_ADDR` | `0.0.0.0:8080` (compose asettaa tämänkin) |
| `JWT_SECRET` | `openssl rand -base64 48` |
| `APP_ENCRYPTION_KEY` | `openssl rand -base64 32` (täsmälleen 32 tavua; jos tämä vaihtuu, Polar-tili on yhdistettävä uudelleen) |
| `COOKIE_SECURE` | `true` (pakollinen HTTPS:n takana; `false` rikkoo kirjautumisen) |
| `PUBLIC_READ` | `true` (näyteikkuna) tai `false` (kaikki vaatii kirjautumisen) |
| `SYNC_INTERVAL_HOURS` | `6` |
| `PUBLIC_BODY_METRICS` | `false` (oletus): paino ja pituus näkyvät vain kirjautuneille. `true` näyttää ne kaikille |
| `ADMIN_EMAIL`, `ADMIN_PASSWORD` | omistajatunnus; luetaan vain ensimmäisellä käynnistyksellä tyhjään kantaan. Vähintään 12 merkkiä. |
| `POLAR_CLIENT_ID`, `POLAR_CLIENT_SECRET` | tuotantoasiakkaan tunnukset |
| `POLAR_REDIRECT_URL` | `https://biometrics.tonikiuru.com/api/polar/callback` |
| `IMAGE_PREFIX`, `IMAGE_TAG` | `ghcr.io/tonzium/biometrics` ja `latest`; vaihda prefix, jos repon nimi on toinen |
| `CLOUDFLARE_TUNNEL_TOKEN` | kohdasta 3 |

## 3. Cloudflare Tunnel

1. https://one.dash.cloudflare.com → **Networks → Tunnels → Create a tunnel** → Cloudflared.
2. Nimi esim. `pve2-biometrics`. Valitse **Docker**-asennusohje ja kopioi siitä pelkkä token
   (pitkä merkkijono `--token` -parametrin perästä). Liitä se `deploy/.env`-tiedostoon
   `CLOUDFLARE_TUNNEL_TOKEN=`-riville. Älä aja Cloudflaren ehdottamaa `docker run` -komentoa;
   compose käynnistää cloudflaredin.
3. **Public Hostname** -välilehti → Add a public hostname:
   - Subdomain `biometrics`, Domain `tonikiuru.com`
   - Type **HTTP**, URL **`web:80`** (compose-verkon palvelunimi, ei localhost)
   Cloudflare luo CNAME-tietueen automaattisesti.
4. Cloudflare → tonikiuru.com → **SSL/TLS**: Encryption mode **Full**. **Edge Certificates →
   Always Use HTTPS** päälle.
5. Valinnainen lisäsuoja: **Access → Applications → Add** → Self-hosted, domain
   `biometrics.tonikiuru.com`, policy joka sallii vain oman sähköpostisi. Tällöin koko sivusto
   on Cloudflare-kirjautumisen takana sovelluksen omasta `PUBLIC_READ`-asetuksesta riippumatta.
   Näyteikkunaa varten jätä tämä pois.

## 4. Käynnistys

Imaget tulevat valmiina GitHub Container Registrystä: CI rakentaa ja julkaisee ne jokaisesta
pushista `master`-haaraan tageilla `latest` ja commitin sha. Jos repo on yksityinen, kirjaudu ensin
GitHubin personal access tokenilla, jolla on `read:packages`-oikeus:

```bash
echo "<PAT>" | docker login ghcr.io -u Tonzium --password-stdin
```

Sitten:

```bash
cd ~/polar-data-hub
docker compose -f deploy/docker-compose.yml pull
docker compose -f deploy/docker-compose.yml up -d
```

Pull kestää alle minuutin. Jos haluat rakentaa imaget palvelimella itse (esim. testataksesi
committia, jota ei ole vielä pushattu), lisää build-override:
`docker compose -f deploy/docker-compose.yml -f deploy/docker-compose.build.yml up -d --build`.

Seuraa:

```bash
docker compose -f deploy/docker-compose.yml ps
docker compose -f deploy/docker-compose.yml logs -f api
```

Lokissa pitää näkyä järjestyksessä `migrations applied`, `created initial owner account`,
`listening addr=0.0.0.0:8080`. cloudflaredin lokissa `Registered tunnel connection`.

Tarkistus selaimella:

1. https://biometrics.tonikiuru.com → yleiskuva näyttää "Polar-tiliä ei ole vielä yhdistetty".
2. https://biometrics.tonikiuru.com/api/health → `{"status":"ok","database":"up"}`.
3. https://biometrics.tonikiuru.com/api/docs → Swagger UI.

## 5. Polar-tilin yhdistäminen ja ensimmäinen synkronointi

1. Kirjaudu (oikea yläkulma) omistajatunnuksella.
2. Asetukset → **Yhdistä Polar-tili** → hyväksy Polar Flow'ssa → palaat asetussivulle
   ilmoituksella "Polar-tili yhdistettiin".
3. **Synkronoi nyt**. Raportti näyttää haetut määrät. Polar tarjoaa historiaa vain noin
   28–30 päivää taaksepäin; siitä eteenpäin ajastin täyttää kantaa 6 tunnin välein.
4. Poista `ADMIN_PASSWORD` tiedostosta `deploy/.env` (sitä ei enää lueta) ja aja
   `docker compose -f deploy/docker-compose.yml up -d`, jotta muutos tulee voimaan.

## 6. Ylläpito

**Päivitys** (uusi koodi GitHubissa ja CI vihreä):

```bash
./deploy/deploy.sh            # git pull + docker compose pull + up
BUILD=1 ./deploy/deploy.sh    # sama, mutta rakentaa imaget palvelimella
```

**Tietyn version ajo**: aseta `.env`-tiedostoon `IMAGE_TAG=<commitin lyhyt sha>` ja aja `deploy.sh`.
Paluu edelliseen versioon on sama temppu toiseen suuntaan.

**Varmuuskopio** kantaa (pg_dump, 30 päivän säilytys):

```bash
./deploy/backup.sh                # -> backups/polar-YYYY-MM-DD.sql.gz
```

Cron joka yö klo 03:15 (`crontab -e` käyttäjänä `polar`):

```
15 3 * * * /home/polar/polar-data-hub/deploy/backup.sh /home/polar/backups >> /home/polar/backup.log 2>&1
```

**Uudelleenkäynnistys** hoituu itsestään: kaikilla palveluilla on `restart: unless-stopped` ja
Docker käynnistyy bootissa.

**Lokit**: `docker compose -f deploy/docker-compose.yml logs -f --tail 200 api`

**Synkronointihistoria**: Asetukset-sivu tai `GET /api/sync/runs` kirjautuneena.

## 7. Vianetsintä

| Oire | Syy ja korjaus |
|---|---|
| Cloudflare näyttää 502/530 | `web`-kontti ei ole ylhäällä tai public hostnamen URL ei ole `web:80`. `docker compose ps`, `logs web`. |
| cloudflared-kontti käynnistyy uudelleen | Token puuttuu tai on väärä. `logs cloudflared`. |
| Kirjautuminen onnistuu, mutta seuraava sivu on taas kirjautumaton | `COOKIE_SECURE` on `false` tai selain ei ole HTTPS:n takana. Tuotannossa aina `true`. |
| Polar-yhdistys päättyy "oauth state mismatch" | Redirect URL Polarin asiakkaassa ei täsmää `POLAR_REDIRECT_URL`:iin tai cookie ei kulkenut (edellinen rivi). |
| Yhdistä-nappi antaa 503 | `POLAR_CLIENT_ID`/`SECRET` puuttuvat `.env`:stä. |
| `api` ei käynnisty: "no users exist and ADMIN_EMAIL…" | Ensimmäinen käynnistys ilman admin-muuttujia. Lisää ne ja käynnistä uudelleen. |
| `api` ei käynnisty: "APP_ENCRYPTION_KEY must decode to exactly 32 bytes" | Generoi avain komennolla `openssl rand -base64 32`. |
| `api` ei käynnisty: "connecting to PostgreSQL … invalid port number" | `POSTGRES_PASSWORD` sisältää `/`, `+` tai `=`. Generoi uusi komennolla `openssl rand -hex 24`. Jos kanta ehti alustua vanhalla salasanalla eikä dataa vielä ole: `docker compose -f deploy/docker-compose.yml down -v` ja `up -d`. |
| Synkronointi on `failed` ja virhe mainitsee 429 | Polarin rate limit. Odota `RateLimit-Reset`-ajan verran; ajastin yrittää uudelleen. |
| `pull` antaa `denied` tai `unauthorized` | Repo on yksityinen: `docker login ghcr.io` (kohta 4) tai tee paketit julkisiksi GitHubissa (Packages → package → Settings → Change visibility). |
| `pull` antaa `manifest unknown` | CI ei ole vielä julkaissut imagea tälle tagille: tarkista Actions-välilehti ja `IMAGE_PREFIX`/`IMAGE_TAG`. |
| Build palvelimella kaatuu muistin loppumiseen | Käytä CI:n imageja (oletus) tai lisää kontille RAMia (4 GB). |

## 8. Mitä palvelimella EI ole

- Ei avoimia portteja (ei 80, ei 443). `ss -tlnp` näyttää vain Dockerin sisäiset.
- Ei Let's Encrypt/Certbot-kikkailua: sertifikaatti on Cloudflaren.
- Ei salaisuuksia gitissä: `deploy/.env` on ignoroitu, ja `chmod 600` estää muita käyttäjiä lukemasta sitä.
