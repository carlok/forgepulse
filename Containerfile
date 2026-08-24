FROM node:22-bookworm AS web-build
WORKDIR /build/web
RUN corepack enable && corepack prepare pnpm@11.0.9 --activate
COPY web/package.json web/pnpm-lock.yaml* web/pnpm-workspace.yaml* ./
RUN pnpm install --no-frozen-lockfile
COPY web/ ./
RUN pnpm build

FROM rust:1-bookworm AS server-build
WORKDIR /build
COPY Cargo.toml Cargo.lock* ./
COPY server/Cargo.toml server/Cargo.toml
COPY server/src server/src
COPY server/migrations server/migrations
RUN cargo build --release --package forgepulse-server

FROM debian:bookworm-slim
RUN apt-get update \
    && apt-get install --yes --no-install-recommends curl \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --system --create-home --uid 10001 forgepulse
COPY --from=server-build /build/target/release/forgepulse-server /usr/local/bin/forgepulse-server
COPY --from=web-build /build/web/dist /opt/forgepulse/web
RUN mkdir -p /data && chown forgepulse:forgepulse /data
USER forgepulse
ENV FORGEPULSE_HOST=0.0.0.0 \
    FORGEPULSE_WEB_DIR=/opt/forgepulse/web \
    FORGEPULSE_DB=sqlite:/data/forgepulse.db
VOLUME ["/data"]
EXPOSE 18744
ENTRYPOINT ["/usr/local/bin/forgepulse-server"]
