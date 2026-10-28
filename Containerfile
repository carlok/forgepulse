FROM node:22-bookworm AS web-build
WORKDIR /build/web
RUN corepack enable && corepack prepare pnpm@11.0.9 --activate
COPY web/package.json web/pnpm-lock.yaml* web/pnpm-workspace.yaml* ./
RUN pnpm install --no-frozen-lockfile
COPY web/ ./
RUN pnpm build

FROM docker.io/library/rust:1-bookworm AS server-build
WORKDIR /build
COPY Cargo.toml Cargo.lock* ./
COPY server/Cargo.toml server/Cargo.toml
COPY server/src server/src
COPY server/migrations server/migrations
RUN cargo build --release --package forgepulse-server

FROM debian:bookworm-slim AS runtime-prep
RUN mkdir -p /data && chown 65532:65532 /data

FROM gcr.io/distroless/cc-debian12:nonroot
COPY --from=runtime-prep --chown=65532:65532 /data /data
COPY --from=server-build --chown=65532:65532 \
    /build/target/release/forgepulse-server /app/forgepulse-server
COPY --from=web-build --chown=65532:65532 /build/web/dist /opt/forgepulse/web
ENV FORGEPULSE_HOST=0.0.0.0 \
    FORGEPULSE_WEB_DIR=/opt/forgepulse/web \
    FORGEPULSE_DB=sqlite:/data/forgepulse.db
VOLUME ["/data"]
EXPOSE 18744
HEALTHCHECK --interval=30s --timeout=5s --retries=3 \
    CMD ["/app/forgepulse-server", "healthcheck"]
ENTRYPOINT ["/app/forgepulse-server"]
