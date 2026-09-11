# --- Vaihe 1: käännös -------------------------------------------------------
# SQLX_OFFLINE=true: kyselyt tarkistetaan .sqlx-kansion metadatasta, ei kantaa vasten.
# rust-toolchain.toml jätetään tarkoituksella kopioimatta, jotta imagen oma
# toolchain kelpaa eikä rustup lataa toista versiota buildin aikana.
FROM rust:1.98-bookworm AS builder
WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
COPY migrations ./migrations
COPY .sqlx ./.sqlx

ENV SQLX_OFFLINE=true
RUN cargo build --release -p api --locked

# --- Vaihe 2: ajo -------------------------------------------------------------
FROM debian:bookworm-slim
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates curl \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --system --uid 10001 --create-home app

COPY --from=builder /app/target/release/api /usr/local/bin/api

USER app
ENV BIND_ADDR=0.0.0.0:8080 \
    RUST_LOG=info
EXPOSE 8080
HEALTHCHECK --interval=30s --timeout=3s --start-period=15s \
    CMD curl -fsS http://127.0.0.1:8080/api/health >/dev/null || exit 1

CMD ["api"]
