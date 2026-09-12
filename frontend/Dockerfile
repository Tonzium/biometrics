# --- Vaihe 1: build (Node) -------------------------------------------------
FROM node:24-alpine AS build
WORKDIR /app

COPY package.json package-lock.json ./
RUN npm ci

COPY . .
RUN npm run build

# --- Vaihe 2: nginx ---------------------------------------------------------
# nginx-unprivileged on sama nginx, mutta myös pääprosessi ajaa nginx-käyttäjänä
# (uid 101) eikä rootina. Siksi kontti ei tarvitse yhtään Linux-kyvykkyyttä
# (ks. deploy/docker-compose.yml).
#
# Portti pysyy 80:ssä, vaikka tämä image kuuntelee oletuksena 8080:aa:
# Cloudflaren tunnelin kohde on web:80, eikä sivusto käy alhaalla, kun sitä ei
# tarvitse muuttaa. Ei-root-prosessi saa varata alle 1024:n portin vain, jos
# kontin verkkonimiavaruudessa ip_unprivileged_port_start on nolla, ja se
# asetetaan compose-tiedostossa. Ilman sitä nginx kaatuu heti käynnistyksessä:
# "bind() to 0.0.0.0:80 failed (13: Permission denied)".
FROM nginxinc/nginx-unprivileged:alpine
COPY nginx.conf /etc/nginx/conf.d/default.conf
# Turvaotsakkeet omassa tiedostossaan; nginx.conf sisällyttää tämän joka
# location-lohkossa (ks. kommentit nginx.conf:n alussa).
COPY security-headers.conf /etc/nginx/snippets/security-headers.conf
COPY --from=build /app/dist /usr/share/nginx/html

EXPOSE 80
HEALTHCHECK --interval=30s --timeout=3s CMD wget -qO- http://127.0.0.1/ >/dev/null || exit 1
