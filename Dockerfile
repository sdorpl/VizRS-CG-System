# Obraz VizRS: statyczna binarka (musl) na pustym obrazie (~3 MB), bez powłoki i pakietów.
#
#   docker build -t vizrs .
#   docker run -d -p 8088:8088 -e VIZRS_TOKEN=TWOJ_TOKEN -v vizrs-data:/data vizrs
#
# Serwer z HTTPS (Caddy, Let's Encrypt): deploy/docker-compose.yml, opis w docs/RUNBOOK.md („VPS i Docker”).
# Konfiguracja wyłącznie przez zmienne VIZRS_* (vizrs --help). Dane: wolumen /data (show.json, assets/, backups/, log.jsonl).

FROM rust:1-alpine AS build
# skrót commita do `vizrs --version` (w kontekście nie ma katalogu .git)
ARG VIZRS_GIT_HASH=docker
WORKDIR /src
COPY source/Cargo.toml source/Cargo.lock source/build.rs ./
COPY source/src ./src
COPY source/web ./web
RUN cargo build --release --locked \
 && mkdir -p /out/data \
 && cp target/release/vizrs /out/vizrs \
 && /out/vizrs --version

FROM scratch
COPY --from=build /out/vizrs /vizrs
# pusty katalog danych z właścicielem 10001 – nazwany wolumen przejmie te uprawnienia
COPY --from=build --chown=10001:10001 /out/data /data
USER 10001:10001
ENV VIZRS_BIND=0.0.0.0 \
    VIZRS_PORT=8088 \
    VIZRS_DATA=/data \
    VIZRS_TCP=0
VOLUME /data
EXPOSE 8088
HEALTHCHECK --interval=15s --timeout=5s --start-period=5s --retries=3 CMD ["/vizrs", "--healthcheck"]
ENTRYPOINT ["/vizrs"]
CMD ["serve"]
