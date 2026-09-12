# Kaaviot

Polar Data Hubin kaaviot Excalidraw-muodossa. Tiedostot ovat tavallista JSONia, joten ne
versioituvat gitissä ja niitä voi muokata käsin.

| Tiedosto | Kaavio | Mitä esittää |
|---|---|---|
| [1-arkkitehtuuri.excalidraw.json](1-arkkitehtuuri.excalidraw.json) | Arkkitehtuurikaavio | Ajonaikainen pino (selain → Cloudflare → cloudflared → nginx → axum → Postgres → Polar), sovelluskerrosten sisältö ja julkaisuputki |
| [2-kayttotapaukset.excalidraw.json](2-kayttotapaukset.excalidraw.json) | Käyttötapauskaavio | Toimijat (vierailija, omistaja, Polar AccessLink, ajastin), järjestelmän rajaus ja yhdeksän käyttötapausta |
| [3-sekvenssi.excalidraw.json](3-sekvenssi.excalidraw.json) | Sekvenssikaavio | Kirjautuminen, Polar-yhdistys (OAuth2 authorization code), synkronointi ja julkinen luku viestitasolla |
| [4-vuokaavio-synkronointi.excalidraw.json](4-vuokaavio-synkronointi.excalidraw.json) | Vuokaavio | Synkronointiajon logiikka: lukitus, kuusi askelta, virheiden käsittely ja lopputila ok / partial / failed |

## Avaaminen

- **Selaimessa:** <https://excalidraw.com> → *Open* (kansiokuvake vasemmassa laidassa) → valitse
  tiedosto. Muokkaukset tallennetaan takaisin *Save to…* -toiminnolla.
- **VS Codessa:** asenna laajennus *Excalidraw* (`pomdtr.excalidraw-editor`), joka avaa
  `*.excalidraw.json`-tiedostot suoraan piirtonäkymään.

## Esitykseen

Kuvan saa ulos Excalidrawista *Export image* -toiminnolla (PNG tai SVG). Diojen taustalle
kannattaa valita läpinäkyvä tausta ja 2× skaalaus.

## Päivittäminen

Tiedostot on generoitu skriptillä [scripts/generoi_kaaviot.py](../../scripts/generoi_kaaviot.py)
(vain Pythonin vakiokirjasto):

```bash
python scripts/generoi_kaaviot.py
```

Ajo **ylikirjoittaa** kaikki neljä tiedostoa. Valitse siis jompikumpi tapa:

- **Pieni korjaus tai hienosäätö:** muokkaa kaaviota Excalidrawissa ja tallenna päälle.
  Älä tämän jälkeen aja skriptiä, tai muutos katoaa.
- **Sisältömuutos:** muuta teksti skriptiin ja generoi uudelleen. Näin kaavion lähde pysyy
  yhdessä paikassa ja muutos näkyy diffissä luettavana.

Kaaviot vastaavat [ARKKITEHTUURI.md](../ARKKITEHTUURI.md):n kuvausta. Jos toteutus muuttuu,
muista päivittää sekä dokumentti että kaavio.
