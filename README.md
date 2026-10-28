# ForgePulse

ForgePulse is a self-hosted GitHub traffic-history dashboard built with Rust, Svelte, SQLite, and Podman. It collects the traffic data that GitHub exposes for a limited window, stores it locally, and presents repository-level and aggregate analytics.

## Development

Prerequisites: Podman and `podman compose`.

```sh
cd forgepulse
cp .env.example .env
podman compose -f compose.dev.yml up --build
```

The API is available only at `http://127.0.0.1:18744`; the Vite development UI is at `http://127.0.0.1:18745`. The checkout is bind-mounted into both development containers at `/workspace`, so Rust and Svelte edits are reflected without rebuilding the images. Persistent state is deliberately separate from source code:

| Volume | Purpose |
| --- | --- |
| `forgepulse-data` | SQLite database |
| `forgepulse-cargo` | Rust dependency and build caches |
| `forgepulse-pnpm` | pnpm package store |

Set `FORGEPULSE_GITHUB_TOKEN` to a fine-grained, read-only token and `FORGEPULSE_DEMO=false` before collecting live data. `FORGEPULSE_FILTER=carlok/*` limits collection to public repositories in that owner. The default demo mode makes the UI immediately inspectable without credentials.

## Quality gates

All checks run inside Podman:

```sh
podman compose -f compose.dev.yml run --rm api cargo test --workspace
podman compose -f compose.dev.yml run --rm api cargo llvm-cov --workspace --all-features --fail-under-lines 80
podman compose -f compose.dev.yml run --rm web sh -c 'pnpm install --frozen-lockfile --store-dir /pnpm/store && pnpm test:coverage'
podman compose -f compose.dev.yml run --rm web sh -c 'pnpm install --frozen-lockfile --store-dir /pnpm/store && pnpm test:e2e'
podman compose -f compose.prod.yml build
```

Unit-test coverage is enforced at 80% line coverage on the server and 80% statements, branches, functions, and lines in the client. Browser workflows are intentionally separate from unit-test coverage.

## Production

Production uses the immutable OCI image and only mounts the SQLite data volume:

```sh
cd forgepulse
podman compose -f compose.prod.yml up --build -d
```

On a Mac with a Podman machine, `scripts/deploy.sh` does the same thing in one step: it starts the machine if needed, builds the image, stamps the running version (`branch@sha`) into `/api/health`, recreates the container and waits for it to be healthy.

The server embeds no development source mount. Its static frontend is copied into the production image and is served by the API process. The service is loopback-only by default. Put it behind an HTTPS reverse proxy only if remote access is intentionally required.

The production runtime image is [Distroless](https://github.com/GoogleContainerTools/distroless) (`gcr.io/distroless/cc-debian12:nonroot`): there is no shell, package manager, or `curl`. Container health is checked with `forgepulse-server healthcheck`, which performs an in-process HTTP GET to `/api/health`.

### Debugging the production container

Distroless images cannot be shelled into with `podman exec … sh`. To inspect a running container, use ephemeral debug tooling instead:

```sh
# Podman 5+ (attach a toolbox with shell and common tools)
podman debug forgepulse-app

# Docker Desktop / Docker Engine 23+ (temporary debug sidecar)
docker debug forgepulse-app
```

From the host, you can still curl the published loopback port, read logs, and inspect health:

```sh
curl -sf http://127.0.0.1:18744/api/health
podman logs forgepulse-app
podman inspect forgepulse-app --format '{{json .State.Health}}'
```

For a drop-in alternative base with the same layout, [Docker Hardened Images](https://dhi.io/) publish Distroless-compatible variants when your registry provides them without extra authentication.

## API surface

| Endpoint | Purpose |
| --- | --- |
| `GET /api/health` | Liveness check |
| `GET /api/v1/dashboard?q=&ranking=human_attention\|clone_volume&sort=&dir=&page=&per_page=` | Scoped summaries, KPIs, charts, ranking explanations, and traffic diagnoses |
| `GET /api/v1/repositories/{owner}/{repository}` | Stored repository traffic, referrers, paths, and stars |
| `GET /api/v1/export.jsonl?q=` | One complete, scoped repository record per JSONL line |
| `POST /api/v1/sync` | Trigger an authenticated collection run |
| `GET /api/v1/sync-runs` | Most recent collection outcomes |

Rank and clone share are evaluated within the same `q` scope as the dashboard and export. Equal total-clone counts receive the same competition rank (`1, 2, 2, 4`). The chart includes zero-filled days, and its statistics are calculated from those displayed daily values.

The dashboard defaults to **Human attention**. Its versioned score uses unique views, external referrer uniques, and 30-day star/fork deltas; clone counts are excluded. Missing snapshot history is returned as `null` rather than zero. **Clone volume** preserves the original clone-based ranking. Traffic diagnoses are explanatory only and never alter raw metrics or either score.

## Backup and restore

Stop writes before taking an offline copy, or use SQLite's online backup command from a temporary container. Keep dated backups outside the `forgepulse-data` volume. To restore, stop the service, replace `forgepulse.db` with a validated backup in the data volume, then start the service again.

For a VM migration: stop the local service, make one final consistent SQLite backup, securely copy that database and the private `.env`, update only the host/volume configuration for the VM, and start `compose.prod.yml`. No data-import feature is required: a new installation can instead start a fresh GitHub collection.
