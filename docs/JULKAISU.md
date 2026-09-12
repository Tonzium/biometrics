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

## 6b. Kannan sovellusrooli ja etävarmuuskopio

Nämä kaksi ovat valinnaisia mutta suositeltuja. Kumpikin otetaan käyttöön muuttamalla vain
`deploy/.env`-tiedostoa; ilman niitä kaikki toimii kuten ennen.

### Sovellusrooli (api ei enää yhdistä superuserina)

Sovellus ajaa kyselynsä omalla roolilla, jolla ei ole superuser-oikeuksia. Se ei voi ajaa
`COPY ... FROM PROGRAM` -komentoa (eli SQL:stä käyttöjärjestelmäkomentoihin), lukea palvelimen
tiedostoja, luoda rooleja eikä pudottaa kantaa. Migraatiot se ajaa normaalisti.

1. Luo salasana ja lisää `deploy/.env`-tiedostoon **molemmat** rivit (vain kirjaimia ja numeroita,
   koska arvo upotetaan yhteysosoitteeseen):

   ```bash
   openssl rand -hex 24      # kopioi tuloste DB_APP_PASSWORD:iin
   ```

   ```
   DB_APP_USER=polar_app
   DB_APP_PASSWORD=<äskeinen tuloste>
   ```

2. **Olemassa olevalle kannalle** (eli tälle palvelimelle) aja kertaluonteinen siirto. Kanta on jo
   alustettu, joten init-skripti ei enää aja itseään. Ota ensin varmuuskopio:

   ```bash
   ./deploy/backup.sh
   set -a; . deploy/.env; set +a
   docker compose -f deploy/docker-compose.yml exec -T db \
     psql -U "$POSTGRES_USER" -d "$POSTGRES_DB" \
          -v ON_ERROR_STOP=1 -v role="$DB_APP_USER" -v pw="$DB_APP_PASSWORD" \
          -v db="$POSTGRES_DB" < deploy/sql/app-role-handover.sql
   ```

   Lopussa tulostuu tarkistus: skeeman ja taulujen omistajana pitää olla `polar_app` ja kannan
   omistajana edelleen `polar`.

3. Käynnistä api uudelleen, jotta se ottaa uuden yhteysosoitteen käyttöön:

   ```bash
   docker compose -f deploy/docker-compose.yml up -d
   docker compose -f deploy/docker-compose.yml logs --tail 20 api
   ```

   Lokissa pitää näkyä `migrations applied` ja `listening`. Tarkista vielä kumpi rooli on
   yhteydessä:

   ```bash
   docker compose -f deploy/docker-compose.yml exec -T db \
     psql -U polar -d polar -c "select usename, count(*) from pg_stat_activity where datname='polar' group by usename;"
   ```

**Paluu entiseen**, jos api ei käynnisty: kommentoi `DB_APP_USER` ja `DB_APP_PASSWORD` pois
`deploy/.env`-tiedostosta ja aja `docker compose -f deploy/docker-compose.yml up -d`. Rooli jää
kantaan, mutta api yhdistää jälleen superuserina. Taulut ovat siirron jälkeen `polar_app`:n
omistuksessa, mikä ei estä superuseria tekemästä mitään.

Uusi asennus (tyhjä kanta) ei tarvitse kohtaa 2 lainkaan: rooli syntyy ensimmäisellä
käynnistyksellä, kun muuttujat ovat `.env`:issä.

### Etävarmuuskopio (salattu kopio koneen ulkopuolelle)

Ilman tätä varmuuskopiot ovat samalla levyllä kuin kanta. Kopio salataan ennen lähtöä julkisella
avaimella, joten palvelin ei voi purkaa omia vanhoja varmuuskopioitaan.

1. Asenna työkalut palvelimelle:

   ```bash
   sudo apt-get install -y age rclone
   ```

2. Luo avainpari **omalla koneella, ei palvelimella**:

   ```bash
   age-keygen -o backup.key
   ```

   Tuloste sisältää julkisen avaimen (`age1...`). Talleta `backup.key` salasanojen hallintaan ja
   varmista, että se on muuallakin kuin yhdellä koneella. **Jos yksityinen avain katoaa,
   etäkopioita ei saa enää auki.**

3. Luo kohde. Cloudflare R2 riittää moninkertaisesti (pakattu dump on kilotavuja):
   Cloudflare → R2 → Create bucket, sitten **Manage API tokens** → luo token, jolla on
   lukuoikeus ja kirjoitusoikeus vain tähän ämpäriin.

4. Täytä `deploy/.env` (ks. `deploy/.env.example`, jossa on valmiit esimerkit R2:lle ja SFTP:lle):

   ```
   BACKUP_AGE_RECIPIENT=age1...          # kohdan 2 julkinen avain
   BACKUP_REMOTE=offsite:polar-varmuuskopiot
   RCLONE_CONFIG_OFFSITE_TYPE=s3
   RCLONE_CONFIG_OFFSITE_PROVIDER=Cloudflare
   RCLONE_CONFIG_OFFSITE_REGION=auto
   RCLONE_CONFIG_OFFSITE_ENDPOINT=https://<account-id>.r2.cloudflarestorage.com
   RCLONE_CONFIG_OFFSITE_ACCESS_KEY_ID=...
   RCLONE_CONFIG_OFFSITE_SECRET_ACCESS_KEY=...
   ```

5. Aja käsin ja katso että kopio menee perille:

   ```bash
   ./deploy/backup.sh
   ```

   Viimeinen rivi kertoo lähetetyn tiedoston ja koon. Yöllinen cron-ajo (kohta 6) tekee saman.

6. **Testaa palautus heti.** Varmuuskopio, jota ei ole kerran palautettu, on arvaus. Omalla
   koneella, jossa yksityinen avain on:

   ```bash
   rclone copyto "offsite:polar-varmuuskopiot/polar-2026-09-12.sql.gz.age" ./palautus.age
   age -d -i backup.key palautus.age | gunzip | head -20
   ```

   Tulosteen pitää alkaa `-- PostgreSQL database dump`. Koko palautus menee samalla tavalla
   `psql`-komentoon (ks. `deploy/backup.sh`-tiedoston alun kommentti).

## 7. Vianetsintä

| Oire | Syy ja korjaus |
|---|---|
| Cloudflare näyttää 502/530 | `web`-kontti ei ole ylhäällä tai public hostnamen URL ei ole `web:80`. `docker compose ps`, `logs web`. |
| `web` kaatuu: "bind() to 0.0.0.0:80 failed (13: Permission denied)" | Isäntä ei hyväksynyt `net.ipv4.ip_unprivileged_port_start`-asetusta. nginx ajaa ei-root-käyttäjänä, joten se tarvitsee sen portille 80. Vaihtoehto: `frontend/nginx.conf`:iin `listen 8080`, web-image uudelleen ja tunnelin kohteeksi `web:8080`. |
| `web` kaatuu: `chown("/var/cache/nginx/client_temp", 101) failed` | `IMAGE_TAG` osoittaa vanhaan, root-pohjaiseen web-imageen, jolta nykyinen compose-tiedosto on ottanut kyvykkyydet pois. Poista `IMAGE_TAG`-kiinnitys (oletus `latest`) tai palauta web:n `cap_add`-lohko. |
| cloudflared-kontti käynnistyy uudelleen | Token puuttuu tai on väärä. `logs cloudflared`. |
| Kirjautuminen onnistuu, mutta seuraava sivu on taas kirjautumaton | `COOKIE_SECURE` on `false` tai selain ei ole HTTPS:n takana. Tuotannossa aina `true`. |
| Polar-yhdistys päättyy "oauth state mismatch" | Redirect URL Polarin asiakkaassa ei täsmää `POLAR_REDIRECT_URL`:iin tai cookie ei kulkenut (edellinen rivi). |
| Yhdistä-nappi antaa 503 | `POLAR_CLIENT_ID`/`SECRET` puuttuvat `.env`:stä. |
| `api` ei käynnisty: "no users exist and ADMIN_EMAIL…" | Ensimmäinen käynnistys ilman admin-muuttujia. Lisää ne ja käynnistä uudelleen. |
| `api` ei käynnisty: "APP_ENCRYPTION_KEY must decode to exactly 32 bytes" | Generoi avain komennolla `openssl rand -base64 32`. |
| `api` kaatuu: `password authentication failed for user "polar_app"` | Sovellusrooli on asetettu `.env`:issä, mutta kantaan ei ole luotu roolia. Aja siirto (kohta 6b) tai kommentoi `DB_APP_USER`/`DB_APP_PASSWORD` pois. |
| `deploy.sh`: "aseta sekä DB_APP_USER että DB_APP_PASSWORD" | Vain toinen rivi on täytetty. Molemmat tai ei kumpaakaan. |
| Varmuuskopio: "on vain N tavua (alle 1000)" | `pg_dump` epäonnistui, eikä rikkinäistä kopiota lähetetty ulos. Tarkista `docker compose ps` ja db:n loki. |
| Varmuuskopio: "age puuttuu" tai "rclone puuttuu" | `sudo apt-get install -y age rclone`. |
| `api` ei käynnisty: "connecting to PostgreSQL … invalid port number" | `POSTGRES_PASSWORD` sisältää `/`, `+` tai `=`. Generoi uusi komennolla `openssl rand -hex 24`. Jos kanta ehti alustua vanhalla salasanalla eikä dataa vielä ole: `docker compose -f deploy/docker-compose.yml down -v` ja `up -d`. |
| Synkronointi on `failed` ja virhe mainitsee 429 | Polarin rate limit. Odota `RateLimit-Reset`-ajan verran; ajastin yrittää uudelleen. |
| `pull` antaa `denied` tai `unauthorized` | Repo on yksityinen: `docker login ghcr.io` (kohta 4) tai tee paketit julkisiksi GitHubissa (Packages → package → Settings → Change visibility). |
| `pull` antaa `manifest unknown` | CI ei ole vielä julkaissut imagea tälle tagille: tarkista Actions-välilehti ja `IMAGE_PREFIX`/`IMAGE_TAG`. |
| Build palvelimella kaatuu muistin loppumiseen | Käytä CI:n imageja (oletus) tai lisää kontille RAMia (4 GB). |

## 8. Mitä palvelimella EI ole

- Ei avoimia portteja (ei 80, ei 443). `ss -tlnp` näyttää vain Dockerin sisäiset.
- Ei Let's Encrypt/Certbot-kikkailua: sertifikaatti on Cloudflaren.
- Ei salaisuuksia gitissä: `deploy/.env` on ignoroitu, ja `chmod 600` estää muita käyttäjiä lukemasta sitä.
