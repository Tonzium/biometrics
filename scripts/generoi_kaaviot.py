# -*- coding: utf-8 -*-
"""Generoi Polar Data Hubin kaaviot Excalidraw-muotoon.

    python scripts/generoi_kaaviot.py

Kirjoittaa neljä tiedostoa hakemistoon docs/kaaviot/:

    1-arkkitehtuuri              ajonaikainen pino, sovelluskerrokset, CI/CD
    2-kayttotapaukset            toimijat ja käyttötapaukset (UML)
    3-sekvenssi                  kirjautuminen, OAuth2, synkronointi, julkinen luku
    4-vuokaavio-synkronointi     synkronointiajon logiikka

VAROITUS: ajo ylikirjoittaa tiedostot. Jos olet muokannut kaavioita käsin
Excalidrawissa, muutokset katoavat — tee muutos tähän skriptiin tai älä aja tätä.

Vaatii vain Pythonin vakiokirjaston (testattu 3.12).
Kaavioiden sisällön lähde on docs/ARKKITEHTUURI.md; pidä ne synkassa.
"""
import json
import os

OUT = os.path.normpath(os.path.join(os.path.dirname(os.path.abspath(__file__)),
                                    os.pardir, "docs", "kaaviot"))

CW = {1: 0.55, 2: 0.52, 3: 0.60}   # keskimääräinen merkkileveys / fontSize

_els = []
_n = [0]


def reset():
    del _els[:]


def base(t, x, y, w, h, stroke="#1e1e1e", bg="transparent", **kw):
    _n[0] += 1
    i = _n[0]
    e = {
        "id": "e%05d" % i,
        "type": t,
        "x": round(x, 2), "y": round(y, 2),
        "width": round(w, 2), "height": round(h, 2),
        "angle": 0,
        "strokeColor": stroke,
        "backgroundColor": bg,
        "fillStyle": "solid",
        "strokeWidth": 2,
        "strokeStyle": "solid",
        "roughness": 1,
        "opacity": 100,
        "groupIds": [],
        "frameId": None,
        "roundness": None,
        "seed": 100000 + (i * 7919) % 900000,
        "version": 1,
        "versionNonce": 200000 + (i * 104729) % 900000,
        "isDeleted": False,
        "boundElements": None,
        "updated": 1,
        "link": None,
        "locked": False,
    }
    e.update(kw)
    _els.append(e)
    return e


def rect(x, y, w, h, stroke, bg, dashed=False, sharp=False, sw=2, round_type=3):
    return base("rectangle", x, y, w, h, stroke, bg,
                strokeStyle="dashed" if dashed else "solid", strokeWidth=sw,
                roundness=None if sharp else {"type": round_type})


def ellipse(x, y, w, h, stroke, bg, sw=2):
    return base("ellipse", x, y, w, h, stroke, bg, strokeWidth=sw)


def diamond(x, y, w, h, stroke, bg, sw=2):
    return base("diamond", x, y, w, h, stroke, bg, strokeWidth=sw,
                roundness={"type": 2})


def tsize(s, size, family, lh):
    lines = s.split("\n")
    w = max(len(l) for l in lines) * size * CW[family]
    return w, len(lines) * size * lh


def text(s, x, y, size=12, family=3, color="#1e1e1e", align="left", lh=1.45, cx=None):
    w, h = tsize(s, size, family, lh)
    if cx is not None:
        x = cx - w / 2.0
        align = "center"
    e = base("text", x, y, w, h, color, "transparent")
    e.update({
        "fontSize": size, "fontFamily": family, "text": s,
        "textAlign": align, "verticalAlign": "top",
        "containerId": None, "originalText": s,
        "lineHeight": lh, "autoResize": True,
    })
    return e


def centered(s, x, y, w, h, size=13, family=3, color="#1e1e1e", lh=1.4):
    """Teksti laatikon keskelle (vaaka- ja pystysuunnassa)."""
    _, th = tsize(s, size, family, lh)
    text(s, 0, y + (h - th) / 2.0, size, family, color, lh=lh, cx=x + w / 2.0)


def arrow(x1, y1, x2, y2, color="#495057", dashed=False, head="arrow", sw=2):
    e = base("arrow", x1, y1, abs(x2 - x1), abs(y2 - y1), color, "transparent",
             strokeStyle="dashed" if dashed else "solid", strokeWidth=sw,
             roundness={"type": 2})
    e.update({
        "points": [[0, 0], [round(x2 - x1, 2), round(y2 - y1, 2)]],
        "lastCommittedPoint": None,
        "startBinding": None, "endBinding": None,
        "startArrowhead": None, "endArrowhead": head,
        "elbowed": False,
    })
    return e


def poly(pts, color="#495057", dashed=False, head="arrow", sw=2):
    """Monipisteinen nuoli absoluuttisin koordinaatein."""
    x0, y0 = pts[0]
    rel = [[round(px - x0, 2), round(py - y0, 2)] for px, py in pts]
    xs = [p[0] for p in rel]
    ys = [p[1] for p in rel]
    e = base("arrow", x0, y0, max(xs) - min(xs), max(ys) - min(ys), color, "transparent",
             strokeStyle="dashed" if dashed else "solid", strokeWidth=sw,
             roundness={"type": 2})
    e.update({
        "points": rel,
        "lastCommittedPoint": None,
        "startBinding": None, "endBinding": None,
        "startArrowhead": None, "endArrowhead": head,
        "elbowed": False,
    })
    return e


def line(x1, y1, x2, y2, color="#868e96", dashed=False, sw=2):
    e = base("line", min(x1, x2), min(y1, y2), abs(x2 - x1), abs(y2 - y1), color,
             "transparent", strokeStyle="dashed" if dashed else "solid",
             strokeWidth=sw, roundness={"type": 2})
    e.update({
        "points": [[0, 0], [round(x2 - x1, 2), round(y2 - y1, 2)]],
        "lastCommittedPoint": None,
    })
    e["x"] = round(x1, 2)
    e["y"] = round(y1, 2)
    e["width"] = round(abs(x2 - x1), 2)
    e["height"] = round(abs(y2 - y1), 2)
    return e


def node(x, y, w, h, title, body, stroke, bg, tsz=16, bsz=12):
    """Pieni laatikko: otsikko + runko, sisältö pystysuunnassa keskellä."""
    rect(x, y, w, h, stroke, bg)
    _, th = tsize(title, tsz, 1, 1.25)
    bh = tsize(body, bsz, 3, 1.45)[1] if body else 0
    gap = 10 if body else 0
    top = y + (h - (th + gap + bh)) / 2.0
    cx = x + w / 2.0
    text(title, 0, top, tsz, 1, stroke, lh=1.25, cx=cx)
    if body:
        text(body, 0, top + th + gap, bsz, 3, "#1e1e1e", lh=1.45, cx=cx)


def card(x, y, w, h, title, body, stroke, bg, tsz=18, bsz=12):
    """Iso kortti: otsikko + vasemmalle tasattu runko."""
    rect(x, y, w, h, stroke, bg)
    text(title, x + 16, y + 14, tsz, 1, stroke, lh=1.25)
    text(body, x + 16, y + 14 + tsz * 1.25 + 10, bsz, 3, "#1e1e1e", lh=1.45)


def save(name):
    doc = {
        "type": "excalidraw",
        "version": 2,
        "source": "https://excalidraw.com",
        "elements": list(_els),
        "appState": {"gridSize": None, "viewBackgroundColor": "#ffffff"},
        "files": {},
    }
    path = os.path.join(OUT, name)
    with open(path, "w", encoding="utf-8") as f:
        json.dump(doc, f, ensure_ascii=False, indent=2)
    print("%-44s %4d elementtia" % (name, len(_els)))
    reset()


# --------------------------------------------------------------------- värit
CLIENT = ("#6741d9", "#e5dbff")
EDGE = ("#f08c00", "#ffec99")
WEB = ("#2f9e44", "#b2f2bb")
API = ("#1971c2", "#a5d8ff")
DB = ("#0c8599", "#99e9f2")
EXT = ("#e03131", "#ffc9c9")
GREY = ("#868e96", "#f1f3f5")
DARK = "#1e1e1e"
MUTED = "#495057"


def otsikko(t, alaotsikko):
    text(t, 60, 40, 32, 1, DARK, lh=1.25)
    text(alaotsikko, 62, 90, 15, 2, MUTED, lh=1.3)


# ===========================================================================
# 1. ARKKITEHTUURIKAAVIO
# ===========================================================================
def arkkitehtuuri():
    otsikko(
        "Polar Data Hub \u2014 full stack -web-sovellus",
        "KAMK \u00b7 Web-sovelluskehitys \u00b7 lopputy\u00f6 \u00b7 React 19 + TypeScript 5.9 \u00b7 Rust 1.98 / axum 0.8 \u00b7 "
        "PostgreSQL 18 \u00b7 Docker Compose + Cloudflare Tunnel\n"
        "Polar Flow -kellon data omaan kantaan ja omaan k\u00e4ytt\u00f6liittym\u00e4\u00e4n \u00b7 "
        "https://biometrics.tonikiuru.com")

    # --- 1. ajonaikainen arkkitehtuuri
    text("1 \u00b7 Ajonaikainen arkkitehtuuri", 60, 150, 20, 1, MUTED, lh=1.25)

    rect(610, 170, 940, 460, "#868e96", "transparent", dashed=True, sw=1)
    text("Kotipalvelin (Proxmox LXC) \u00b7 Docker Compose \u00b7 yksi sis\u00e4inen verkko \u00b7 "
         "ei julkaistuja portteja", 628, 186, 16, 1, MUTED, lh=1.25)

    node(60, 250, 230, 130, "Selain",
         "React 19 SPA\nTanStack Query\nRecharts-kaaviot", *CLIENT)
    node(350, 250, 200, 130, "Cloudflare",
         "DNS + TLS\nAlways Use HTTPS\nrate limit (valinn.)", *EDGE)
    node(650, 265, 180, 100, "cloudflared",
         "l\u00e4htev\u00e4 tunneli\nei avoimia portteja", *EDGE)
    node(880, 235, 250, 160, "web \u00b7 nginx:alpine",
         "React-build staattisena\n/ \u2192 index.html (SPA)\n/api/* \u2192 api:8787\n"
         "CSP \u00b7 HSTS \u00b7 nosniff\nlimit_req 10/min login", *WEB)
    node(1185, 225, 320, 180, "api \u00b7 Rust 1.98 + axum 0.8",
         "REST + OpenAPI (/api/docs)\nauth \u00b7 polar \u00b7 sync \u00b7 data\n"
         "argon2id \u00b7 JWT \u00b7 AES-256-GCM\nsynkronointi + ajastin\n"
         "ei-root uid 10001, cap_drop ALL", *API)
    node(1215, 460, 260, 130, "db \u00b7 PostgreSQL 18",
         "taulut + raw jsonb\nn\u00e4kym\u00e4t yhteenvedoille\nvolume pgdata", *DB)
    node(1615, 245, 300, 140, "Polar AccessLink API",
         "flow.polar.com \u2014 OAuth2\nwww.polaraccesslink.com/v3\n"
         "exercises \u00b7 sleep \u00b7 recharge\nactivity \u00b7 physical \u00b7 cardio-load", *EXT)

    node(650, 440, 500, 170, "Synkronointi (api-kontin sis\u00e4ll\u00e4)",
         "K\u00e4ynnistys: POST /api/sync (omistaja) tai ajastin\n"
         "Yksi ajo kerrallaan \u2014 tokio Mutex::try_lock\n"
         "6 askelta: exercises \u00b7 sleep \u00b7 nightly-recharge \u00b7\n"
         "activities \u00b7 physical-info \u00b7 cardio-load\n"
         "Askeleen virhe lokiin ja jatketaan; Polarin 429 keskeytt\u00e4\u00e4\n"
         "Tulos sync_runs-tauluun: ok / partial / failed",
         "#1971c2", "#e7f5ff")
    node(1615, 420, 300, 165, "Polar-yhdistys (OAuth2)",
         "1  /api/polar/connect \u2192 state-cookie\n"
         "2  k\u00e4ytt\u00e4j\u00e4 hyv\u00e4ksyy Polarissa\n"
         "3  /api/polar/callback?code&state\n"
         "4  code \u2192 token, POST /v3/users\n"
         "5  token AES-256-GCM \u2192 kantaan",
         "#e03131", "#fff0f0")

    arrow(292, 315, 348, 315)
    text("HTTPS", 0, 288, 12, 2, MUTED, cx=320, lh=1.25)
    arrow(552, 315, 648, 315)
    text("tunneli", 0, 288, 12, 2, MUTED, cx=600, lh=1.25)
    arrow(832, 315, 878, 315)
    text("http", 0, 288, 12, 2, MUTED, cx=855, lh=1.25)
    arrow(1132, 315, 1183, 315)
    text("/api/*", 0, 288, 12, 2, MUTED, cx=1157, lh=1.25)
    arrow(1507, 315, 1613, 315)
    text("HTTPS", 0, 288, 12, 2, MUTED, cx=1560, lh=1.25)
    arrow(1345, 407, 1345, 458)
    text("sqlx", 1358, 420, 12, 2, MUTED, lh=1.25)
    arrow(1152, 478, 1186, 400, "#868e96", dashed=True, head=None, sw=1)

    # --- 2. sovelluskerrokset
    text("2 \u00b7 Sovelluskerrokset", 60, 660, 20, 1, MUTED, lh=1.25)

    card(60, 700, 400, 450, "Frontend \u00b7 React 19 + TypeScript",
         "Vite 8 -build \u2192 staattiset tiedostot nginxiin\n"
         "\n"
         "Sivut (react-router)\n"
         "  Dashboard \u00b7 Exercises \u00b7 ExerciseDetail\n"
         "  Sleep \u00b7 Activity \u00b7 Settings \u00b7 Login \u00b7 404\n"
         "\n"
         "Komponentit\n"
         "  Layout \u00b7 StatCard \u00b7 RangePicker \u00b7 Status\n"
         "  charts.tsx \u2014 Recharts-kuvaajat\n"
         "\n"
         "src/api\n"
         "  client.ts    fetch-k\u00e4\u00e4re, credentials\n"
         "  hooks.ts     TanStack Query: v\u00e4limuisti\n"
         "  schema.d.ts  generoitu /api/openapi.json:sta\n"
         "\n"
         "Testit: Vitest + Testing Library",
         "#6741d9", "#f3f0ff")

    card(490, 700, 545, 450, "Backend \u00b7 Rust + axum",
         "Cargo-workspace, kolme cratea\n"
         "\n"
         "domain        User, Role, DomainError \u2014 ei HTTP:t\u00e4 eik\u00e4 kantaa\n"
         "polar-client  OAuth2 + /v3-reitit \u2014 testit wiremockia vasten\n"
         "api           axum-palvelin:\n"
         "  config.rs   env \u2192 Config, kaatuu heti jos muuttuja puuttuu\n"
         "  state.rs    pool, cipher, polar-client, sync-lukko\n"
         "  auth/       argon2id, JWT, CurrentUser / ReadAccess\n"
         "  crypto.rs   AES-256-GCM Polar-tokenille\n"
         "  db/         sqlx::query! \u2014 SQL tarkistetaan k\u00e4\u00e4nn\u00f6saikana\n"
         "  sync/       synkronointimoottori + ajastin\n"
         "  routes/     health \u00b7 auth \u00b7 polar \u00b7 sync \u00b7 data + OpenAPI\n"
         "\n"
         "Suojaus on ekstraktorissa, ei middlewaressa: reitti joka ottaa\n"
         "CurrentUser-parametrin ei voi unohtaa tarkistusta.\n"
         "\n"
         "Julkiset    /api/health \u00b7 /api/meta \u00b7 /api/auth/login \u00b7 /docs\n"
         "Lukureitit  /api/exercises \u00b7 /sleep \u00b7 /recharge \u00b7 /activity \u00b7\n"
         "            /cardio-load \u00b7 /physical \u00b7 /summary/*  (ReadAccess)\n"
         "Omistaja    /api/polar/* \u00b7 /api/sync \u00b7 /api/auth/me  (owner)\n"
         "\n"
         "Testit: cargo test \u00b7 #[sqlx::test] \u00b7 tower::oneshot \u00b7 wiremock",
         "#1971c2", "#e7f5ff")

    card(1065, 700, 420, 450, "Tietokanta \u00b7 PostgreSQL 18",
         "app_users \u2500\u25000..1\u2500\u2500 polar_accounts\n"
         "                  \u251c\u2500\u2500 exercises\n"
         "                  \u251c\u2500\u2500 sleep_nights      (PK tili+pvm)\n"
         "                  \u251c\u2500\u2500 nightly_recharge  (PK tili+pvm)\n"
         "                  \u251c\u2500\u2500 daily_activity    (PK tili+pvm)\n"
         "                  \u251c\u2500\u2500 physical_info     (PK tili+pvm)\n"
         "                  \u251c\u2500\u2500 cardio_load       (PK tili+pvm)\n"
         "                  \u2514\u2500\u2500 sync_runs\n"
         "\n"
         "Jokaisessa Polar-taulussa on purettu sarakemuoto\n"
         "kyselyj\u00e4 varten JA raw jsonb \u2014 Polarin vastaus\n"
         "s\u00e4ilyy sellaisenaan.\n"
         "\n"
         "Upsertit ON CONFLICT ... DO UPDATE tekev\u00e4t\n"
         "synkronoinnista idempotentin.\n"
         "\n"
         "N\u00e4kym\u00e4t\n"
         "  v_daily_wellness   uni + palautuminen +\n"
         "                     aktiivisuus + kuorma / p\u00e4iv\u00e4\n"
         "  v_weekly_summary   harjoittelu + uni / viikko\n"
         "\n"
         "Migraatiot 0001\u20130006 ajetaan k\u00e4ynnistyksess\u00e4.",
         "#0c8599", "#e3fafc")

    card(1515, 700, 400, 450, "Autentikointi & tietoturva",
         "Kirjautuminen\n"
         "  POST /api/auth/login \u2192 argon2id-vertailu\n"
         "  JWT HS256 (7 vrk) httpOnly-cookie pdh_session\n"
         "  SameSite=Lax + Secure \u2192 CSRF-suoja ilman tokenia\n"
         "  token_version kannassa \u2192 istunnot mit\u00e4t\u00f6it\u00e4viss\u00e4\n"
         "\n"
         "Tulvasuoja (kaksi kerrosta)\n"
         "  nginx limit_req 10/min per IP\n"
         "  enint. 2 rinnakkaista argon2-laskentaa \u2192 429\n"
         "\n"
         "Roolit\n"
         "  owner   yhdist\u00e4\u00e4 Polarin ja synkronoi\n"
         "  viewer  lukee   (rekister\u00f6inti\u00e4 ei ole)\n"
         "\n"
         "Muu suojaus\n"
         "  Polar-token levossa AES-256-GCM\n"
         "  sqlx::query! \u2192 ei dynaamista SQL:\u00e4\u00e4\n"
         "  CSP \u00b7 HSTS \u00b7 nosniff \u00b7 X-Frame-Options: DENY\n"
         "  no-new-privileges \u00b7 cap_drop ALL \u00b7 uid 10001\n"
         "  ei julkaistuja portteja \u2014 TLS Cloudflaressa",
         "#e03131", "#fff5f5")

    # --- 3. julkaisuputki
    text("3 \u00b7 Julkaisuputki (CI/CD)", 60, 1185, 20, 1, MUTED, lh=1.25)

    node(60, 1225, 250, 135, "git push",
         "master-haara\nDependabot-PR:t\nactionit SHA-pinnattu", *GREY)
    node(350, 1225, 430, 135, "GitHub Actions",
         "cargo fmt \u00b7 clippy -D warnings\n"
         "cargo test \u2014 Postgres-palvelu, SQLX_OFFLINE\n"
         "vitest \u00b7 typecheck \u00b7 lint \u00b7 vite build\n"
         "cargo audit \u00b7 npm audit \u00b7 compose-validointi", *GREY)
    node(820, 1225, 340, 135, "Docker build \u2192 GHCR",
         "ghcr.io/\u2026-api ja \u2026-web\ntagit: latest + commitin sha\n"
         "(Rust-build ei mahdu palvelimelle)", *GREY)
    node(1200, 1225, 400, 135, "Palvelin vet\u00e4\u00e4 imaget",
         "./deploy/deploy.sh\ndocker compose pull && up -d\nrollback: IMAGE_TAG=<sha>", *GREY)
    node(1640, 1225, 275, 135, "Ajossa",
         "4 konttia, 1 verkko\nhealthcheckit\nlokit: json-file + rotaatio", *WEB)

    arrow(312, 1292, 348, 1292)
    arrow(782, 1292, 818, 1292)
    arrow(1162, 1292, 1198, 1292)
    arrow(1602, 1292, 1638, 1292)

    save("1-arkkitehtuuri.excalidraw.json")


# ===========================================================================
# 2. KÄYTTÖTAPAUSKAAVIO
# ===========================================================================
def tikku(x, y, nimi, stroke="#6741d9"):
    """UML-toimija tikku-ukkona. x,y = pään vasen ylakulma."""
    ellipse(x + 18, y, 36, 36, stroke, "transparent")
    line(x + 36, y + 36, x + 36, y + 92, stroke)
    line(x + 4, y + 54, x + 68, y + 54, stroke)
    line(x + 36, y + 92, x + 8, y + 134, stroke)
    line(x + 36, y + 92, x + 64, y + 134, stroke)
    text(nimi, 0, y + 146, 14, 1, stroke, lh=1.3, cx=x + 36)


def kayttotapaukset():
    otsikko("Polar Data Hub \u2014 k\u00e4ytt\u00f6tapauskaavio",
            "Toimijat, j\u00e4rjestelm\u00e4n rajaus ja k\u00e4ytt\u00f6tapaukset \u00b7 "
            "roolit owner ja viewer \u00b7 rekister\u00f6itymist\u00e4 ei ole")

    rect(520, 190, 580, 1000, "#868e96", "transparent", dashed=True, sw=1)
    text("Polar Data Hub  (j\u00e4rjestelm\u00e4)", 0, 206, 17, 1, MUTED, lh=1.25, cx=810)

    kt = [
        "Selaa koontin\u00e4ytt\u00f6\u00e4",
        "Tarkastele harjoituksia",
        "Tarkastele unta ja palautumista",
        "Tarkastele aktiivisuutta",
        "Kirjaudu sis\u00e4\u00e4n",
        "Yhdist\u00e4 Polar-tili (OAuth2)",
        "K\u00e4ynnist\u00e4 synkronointi",
        "Nouda data Polar AccessLinkist\u00e4",
        "Katso synkronoinnin tila",
    ]
    varit = [WEB, WEB, WEB, WEB, CLIENT, CLIENT, CLIENT, EXT, CLIENT]
    ys = []
    for i, (label, (st, bg)) in enumerate(zip(kt, varit)):
        y = 260 + i * 96 + (48 if i >= 7 else 0)   # tilaa «include»-nuolelle
        ys.append(y + 38)
        ellipse(610, y, 400, 76, st, bg)
        centered(label, 610, y, 400, 76, 15, 1, DARK, lh=1.25)

    tikku(200, 330, "Vierailija\n(ei kirjautunut)", "#2f9e44")
    tikku(200, 760, "Omistaja\n(rooli: owner)", "#6741d9")

    node(1200, 700, 260, 90, "Polar AccessLink",
         "\u00abulkoinen j\u00e4rjestelm\u00e4\u00bb", *EXT, tsz=15, bsz=11)
    node(1200, 950, 260, 90, "Ajastin",
         "SYNC_INTERVAL_HOURS", *EDGE, tsz=15, bsz=11)

    for i in (0, 1, 2, 3):
        line(272, 397, 610, ys[i], "#868e96", sw=1)
    for i in (4, 5, 6, 8):
        line(272, 827, 610, ys[i], "#868e96", sw=1)
    line(1200, 745, 1010, ys[5], "#868e96", sw=1)
    line(1200, 745, 1010, ys[7], "#868e96", sw=1)
    line(1200, 995, 1010, ys[7], "#868e96", sw=1)

    # yleistys: omistaja perii vierailijan oikeudet (kierret\u00e4\u00e4n nimitekstien ohi)
    poly([(204, 827), (150, 827), (150, 397), (196, 397)], "#868e96", head="triangle", sw=1)
    text("perii", 158, 596, 13, 2, MUTED, lh=1.25)

    # include-suhde
    arrow(810, 914, 810, 976, "#495057", dashed=True)
    text("\u00abinclude\u00bb", 822, 930, 13, 2, MUTED, lh=1.25)

    card(520, 1240, 940, 140, "Huomiot",
         "PUBLIC_READ=true (oletus): lukuk\u00e4ytt\u00f6tapaukset 1\u20134 ovat auki ilman kirjautumista \u2014 sivusto on\n"
         "julkinen n\u00e4yteikkuna. PUBLIC_READ=false sulkee kaiken kirjautumisen taakse.\n"
         "Rekister\u00f6itymist\u00e4 ei ole: omistaja luodaan ymp\u00e4rist\u00f6muuttujista ensimm\u00e4isell\u00e4 k\u00e4ynnistyksell\u00e4.\n"
         "Paino ja pituus n\u00e4kyv\u00e4t vain kirjautuneelle (PUBLIC_BODY_METRICS=false).",
         "#868e96", "#f8f9fa", tsz=16, bsz=13)

    save("2-kayttotapaukset.excalidraw.json")


# ===========================================================================
# 3. SEKVENSSIKAAVIO
# ===========================================================================
def sekvenssi():
    otsikko("Polar Data Hub \u2014 sekvenssikaavio",
            "Kirjautuminen \u2192 Polar-yhdistys (OAuth2) \u2192 synkronointi \u2192 julkinen luku \u00b7 "
            "yhten\u00e4inen nuoli = pyynt\u00f6, katkoviiva = vastaus")

    L = {  # lifeline x
        "sel": 170, "web": 500, "api": 830, "db": 1180, "pol": 1510,
    }
    heads = [
        ("sel", "Selain", CLIENT),
        ("web", "web \u00b7 nginx", WEB),
        ("api", "api \u00b7 axum", API),
        ("db", "db \u00b7 Postgres", DB),
        ("pol", "Polar AccessLink", EXT),
    ]
    for key, label, (st, bg) in heads:
        x = L[key]
        rect(x - 105, 165, 210, 64, st, bg)
        centered(label, x - 105, 165, 210, 64, 16, 1, st, lh=1.25)

    bands = [
        ("1 \u00b7 Kirjautuminen", "#e03131", "#fff5f5", [
            ("m", "sel", "web", "POST /api/auth/login {email, salasana}"),
            ("m", "web", "api", "limit_req 10/min per IP \u2192 v\u00e4lit\u00e4 ylavirtaan"),
            ("m", "api", "db", "SELECT app_users WHERE email"),
            ("r", "db", "api", "password_hash, rooli, token_version"),
            ("s", "api", "argon2id-vertailu spawn_blockingissa\n"
                         "(enint. 2 rinnakkain, muuten 429 + Retry-After)"),
            ("r", "api", "sel", "200 OK + Set-Cookie: pdh_session  (httpOnly, SameSite=Lax, 7 vrk)"),
        ]),
        ("2 \u00b7 Polar-yhdistys (OAuth2 authorization code)", "#f08c00", "#fff9db", [
            ("i", "nginx v\u00e4litt\u00e4\u00e4 /api/*-pyynn\u00f6t my\u00f6s t\u00e4ss\u00e4 osassa \u2014 j\u00e4tetty pois selkeyden vuoksi"),
            ("m", "sel", "api", "GET /api/polar/connect   (pdh_session-cookie)"),
            ("r", "api", "sel", "303 \u2192 flow.polar.com  +  state-cookie (10 min)"),
            ("m", "sel", "pol", "k\u00e4ytt\u00e4j\u00e4 hyv\u00e4ksyy oikeudet Polarissa"),
            ("r", "pol", "sel", "303 \u2192 /api/polar/callback?code&state"),
            ("m", "sel", "api", "GET /api/polar/callback?code&state"),
            ("s", "api", "vertaa state-parametria cookieen (CSRF)\ncookie poistetaan aina"),
            ("m", "api", "pol", "POST /oauth2/token   (Basic auth: client id + secret)"),
            ("r", "pol", "api", "access_token"),
            ("m", "api", "pol", "POST /v3/users   (409 = jo rekister\u00f6ity)"),
            ("m", "api", "db", "INSERT polar_accounts  (token AES-256-GCM)"),
            ("r", "api", "sel", "303 \u2192 /settings?polar=connected"),
        ]),
        ("3 \u00b7 Synkronointi \u2014 manuaalisesti tai ajastimesta", "#1971c2", "#e7f5ff", [
            ("m", "sel", "api", "POST /api/sync      (tai ajastin laukeaa)"),
            ("s", "api", "Mutex::try_lock \u2014 yksi ajo kerrallaan\nmuuten 409 Conflict"),
            ("m", "api", "db", "SELECT token \u2192 pura AES-256-GCM"),
            ("loop", "jokaiselle 6 askeleesta: exercises \u00b7 sleep \u00b7 nightly-recharge \u00b7 "
                     "activities \u00b7 physical-info \u00b7 cardio-load"),
            ("m", "api", "pol", "GET /v3/exercises, /sleep, /nightly-recharge, \u2026"),
            ("r", "pol", "api", "JSON-alkiot  (429 keskeytt\u00e4\u00e4 koko ajon)"),
            ("m", "api", "db", "UPSERT \u2026 ON CONFLICT DO UPDATE  (idempotentti)"),
            ("end", ""),
            ("m", "api", "db", "INSERT sync_runs: ok / partial / failed"),
            ("r", "api", "sel", "200 OK: tila, rivim\u00e4\u00e4r\u00e4t ja virheet"),
        ]),
        ("4 \u00b7 Julkinen luku (PUBLIC_READ=true)", "#2f9e44", "#ebfbee", [
            ("m", "sel", "web", "GET /api/summary/overview"),
            ("m", "web", "api", "/api/* \u2192 api:8787   (ReadAccess p\u00e4\u00e4st\u00e4\u00e4 l\u00e4pi)"),
            ("m", "api", "db", "SELECT v_daily_wellness, v_weekly_summary"),
            ("r", "api", "sel", "200 OK: ei tunnisteita eik\u00e4 raakaa JSONia"),
        ]),
    ]

    STEP = {"m": 48, "r": 48, "s": 78, "i": 30, "loop": 44, "end": 22}
    y = 270
    for title, color, bg, msgs in bands:
        h = 46 + 14 + sum(STEP[m[0]] for m in msgs)
        rect(70, y, 1560, h, color, bg, dashed=True, sw=1)
        text(title, 86, y + 12, 16, 1, color, lh=1.25)
        yy = y + 60
        loop_top = [0, ""]
        for m in msgs:
            if m[0] == "i":
                text(m[1], 110, yy - 18, 12, 2, MUTED, lh=1.25)
                yy += STEP["i"]
                continue
            if m[0] == "loop":
                loop_top = [yy - 26, m[1]]
                yy += STEP["loop"]
                continue
            if m[0] == "end":
                top = loop_top[0]
                rect(300, top, 1300, (yy - 30) - top, "#868e96", "transparent",
                     dashed=True, sw=1, sharp=True)
                rect(300, top, 74, 24, "#868e96", "#ffffff", sw=1, sharp=True)
                centered("loop", 300, top, 74, 24, 12, 3, MUTED, lh=1.25)
                text("[ " + loop_top[1] + " ]", 386, top + 4, 12, 3, MUTED, lh=1.25)
                yy += STEP["end"]
                continue
            if m[0] == "s":
                x = L[m[1]]
                poly([(x, yy), (x + 78, yy), (x + 78, yy + 34), (x + 8, yy + 34)], DARK)
                text(m[2], x + 92, yy + 2, 12, 3, DARK, lh=1.3)
                yy += STEP["s"]
                continue
            x1, x2 = L[m[1]], L[m[2]]
            d = 1 if x2 > x1 else -1
            arrow(x1 + 4 * d, yy, x2 - 4 * d, yy, DARK, dashed=(m[0] == "r"))
            text(m[3], min(x1, x2) + 14, yy - 22, 12, 3, DARK, lh=1.3)
            yy += STEP[m[0]]
        y += h + 26

    for key in L:
        line(L[key], 229, L[key], y - 10, "#adb5bd", dashed=True, sw=1)

    save("3-sekvenssi.excalidraw.json")


# ===========================================================================
# 4. VUOKAAVIO — SYNKRONOINTI
# ===========================================================================
def vuokaavio():
    otsikko("Polar Data Hub \u2014 synkronoinnin vuokaavio",
            "POST /api/sync tai ajastin \u00b7 py\u00f6ristetty = alku/loppu \u00b7 suorakaide = toiminto \u00b7 "
            "vinoneli\u00f6 = p\u00e4\u00e4t\u00f6s")

    CX = 900          # p\u00e4\u00e4linjan keskikohta
    BX, BW = 660, 480  # toimintolaatikot
    DX, DW = 630, 540  # p\u00e4\u00e4t\u00f6kset
    LX, LW = 90, 420   # vasen sarake

    def box(y, h, s, stroke="#1971c2", bg="#e7f5ff", rounded=False):
        rect(BX, y, BW, h, stroke, bg, round_type=3 if rounded else 3,
             sharp=not rounded and False)
        centered(s, BX, y, BW, h, 13, 3, DARK)

    def dia(y, h, s):
        diamond(DX, y, DW, h, "#f08c00", "#fff9db")
        centered(s, DX, y, DW, h, 13, 3, DARK)

    def side(x, y, w, h, s, stroke, bg):
        rect(x, y, w, h, stroke, bg)
        centered(s, x, y, w, h, 13, 3, DARK)

    def haara(x1, y1, x2, y2, label, lx, ly):
        arrow(x1, y1, x2, y2)
        text(label, lx, ly, 13, 2, MUTED, lh=1.25)

    # alku
    rect(BX, 150, BW, 76, "#6741d9", "#e5dbff", round_type=3)
    centered("POST /api/sync  (omistaja)\ntai ajastin (SYNC_INTERVAL_HOURS)", BX, 150, BW, 76,
             13, 3, DARK)
    arrow(CX, 226, CX, 258)

    dia(260, 130, "Ajo jo k\u00e4ynniss\u00e4?\n(tokio Mutex::try_lock)")
    haara(630, 325, 514, 325, "kyll\u00e4", 540, 300)
    side(LX, 285, LW, 80, "409 Conflict\nsynkronointi on jo k\u00e4ynniss\u00e4", *EXT)
    haara(CX, 390, CX, 428, "ei", 912, 398)

    box(430, 90, "Hae polar_accounts \u2014 pura token (AES-256-GCM)\n"
                 "Avaa sync_runs-rivi")
    arrow(CX, 520, CX, 558)

    dia(560, 120, "Polar-tili yhdistetty?")
    haara(630, 620, 514, 620, "ei", 540, 595)
    side(LX, 580, LW, 80, "503 Service Unavailable\nPolar-tili\u00e4 ei ole yhdistetty", *EXT)
    haara(CX, 680, CX, 718, "kyll\u00e4", 912, 688)

    box(720, 110, "Ota seuraava askel (6 kpl):\n"
                  "exercises \u00b7 sleep \u00b7 nightly-recharge\n"
                  "activities \u00b7 physical-info \u00b7 cardio-load",
        "#0c8599", "#e3fafc")
    arrow(CX, 830, CX, 868)

    box(870, 70, "GET Polar AccessLink -reitti  (access_token)")
    arrow(CX, 940, CX, 978)

    dia(980, 120, "Vastaus 429?")
    haara(1170, 1040, 1246, 1040, "kyll\u00e4", 1178, 1010)
    side(1250, 995, 230, 90, "Keskeyt\u00e4 ajo\nsync_runs = failed", *EXT)
    haara(CX, 1100, CX, 1138, "ei", 912, 1108)

    dia(1140, 130, "Askel onnistui?\n(HTTP-vastaus + j\u00e4sennys)")
    haara(630, 1205, 514, 1205, "ei", 540, 1180)
    side(LX, 1165, LW, 80, "Kirjaa virhe askeleelle\najon tila \u2192 partial", *EDGE)
    haara(CX, 1270, CX, 1308, "kyll\u00e4", 912, 1278)

    box(1310, 90, "J\u00e4sennä alkiot \u2014 virheellinen ohitetaan varoituksella\n"
                  "UPSERT kantaan: ON CONFLICT \u2026 DO UPDATE",
        "#0c8599", "#e3fafc")
    arrow(CX, 1400, CX, 1468)
    poly([(300, 1245), (300, 1440), (CX, 1440)], "#f08c00")

    dia(1470, 120, "Askeleita j\u00e4ljell\u00e4?")
    poly([(1170, 1530), (1230, 1530), (1230, 775), (1144, 775)], "#0c8599")
    text("kyll\u00e4", 1180, 1498, 13, 2, MUTED, lh=1.25)
    haara(CX, 1590, CX, 1628, "ei", 912, 1598)

    dia(1630, 120, "Virheit\u00e4 kirjattu?")
    haara(630, 1690, 514, 1690, "kyll\u00e4", 528, 1665)
    side(LX, 1650, LW, 80, "sync_runs = partial", *EDGE)
    haara(CX, 1750, CX, 1788, "ei", 912, 1758)

    box(1790, 70, "sync_runs = ok", "#2f9e44", "#b2f2bb")
    arrow(CX, 1860, CX, 1898)

    rect(BX, 1900, BW, 86, "#6741d9", "#e5dbff", round_type=3)
    centered("200 OK: tila, rivim\u00e4\u00e4r\u00e4t ja virheet\n"
             "(ok / partial / failed) \u2014 tulos j\u00e4\u00e4 sync_runs-tauluun",
             BX, 1900, BW, 86, 13, 3, DARK)

    poly([(300, 1730), (300, 1943), (BX - 4, 1943)], "#f08c00")
    poly([(1365, 1085), (1365, 1943), (BX + BW + 4, 1943)], "#e03131")

    save("4-vuokaavio-synkronointi.excalidraw.json")


if not os.path.isdir(OUT):
    os.makedirs(OUT)

arkkitehtuuri()
kayttotapaukset()
sekvenssi()
vuokaavio()
