# Obraz dla Render.com (render.yaml). Budowa z katalogu głównego: docker build -f deploy/render/Dockerfile -t vizrs-render .
FROM rust:1-slim-bookworm AS build
WORKDIR /src
COPY source/ .
RUN cargo build --release

FROM debian:bookworm-slim
COPY --from=build /src/target/release/vizrs /usr/local/bin/vizrs
COPY deploy/render/start.sh /usr/local/bin/vizrs-start
COPY deploy/render/seed/ /usr/local/share/vizrs-seed/
RUN chmod +x /usr/local/bin/vizrs-start
ENV VIZRS_DATA=/var/data
EXPOSE 10000
CMD ["vizrs-start"]
