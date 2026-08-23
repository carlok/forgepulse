# ForgePulse delivery sprints

## Product boundary

ForgePulse is a new, Podman-first traffic-history application: Rust/Axum on the server; Svelte 5, custom CSS, and ECharts in the browser; and SQLite/WAL for durable local storage. Development mounts the host checkout at `/workspace`; production has no source mount. The first release collects public `carlok/*` repositories from GitHub and starts from a fresh database.

## Sprint 1 — Foundation and developer loop

- Establish the Rust workspace, Axum health endpoint, Svelte/Vite shell, and design-token CSS.
- Provide development and production OCI Containerfiles, loopback-only ports, host source bind mounts, and only persistent data/cache named volumes.
- Add Rust unit testing, `cargo llvm-cov`, Vitest V8 coverage, Playwright workflow configuration, and CI image builds.
- Exit: editing host code hot-reloads through Podman; unit and coverage commands run entirely in containers.

## Sprint 2 — Collection and persistence

- Apply SQLite migrations for repositories, daily traffic, referrer and path snapshots, stars, and sync-run status.
- Add token/configuration loading, owner/repository filter grammar, GitHub collection, bounded retries, rate-limit backoff, scheduler, and restart-safe upserts.
- Cover GitHub fixtures, persistence, filtering, restarts, and failed synchronizations in server tests.
- Exit: a clean `carlok/*` collection preserves history through service restarts with server line coverage at or above 80%.

## Sprint 3 — Analytics API

- Provide repository search, sorting, pagination, KPI totals, detail data, zero-filled clone history, and a documented health/sync surface.
- Calculate total-clone and unique-cloner daily statistics server-side: mean, median, population variance, population standard deviation, min, max, and nearest-rank P95.
- Add scoped `# - %` ranking/share and JSONL export. Each line carries a summary, rank/share, all stored clones/views, referrers, paths, and stars for the current search scope.
- Exit: API tests cover empty/zero-filled series, descriptive statistics, ties, export shape, and query scope.

## Sprint 4 — Svelte dashboard

- Build responsive KPIs, a searchable repository table, `# - %` as its leftmost column, detail views, and persisted dark/light mode.
- Build the clone chart with total-clone and unique-cloner ECharts series, an interactive legend, and a total/unique selector for the statistics cards.
- Add JSONL download and test transforms, filters, selectors, ranking display, export state, themes, and browser workflows.
- Exit: the dashboard is accessible and responsive; the client coverage gate passes.

## Sprint 5 — Operational release

- Add verified SQLite backup/restore instructions, structured logs, Prometheus metrics, local and VM runbooks, and an immutable production image.
- Exit: a restored backup is valid and production starts without the development checkout mounted.

## Acceptance checklist

- Public GitHub collection is fresh; no database import is included.
- The dashboard preserves the requested traffic-analysis sense: clone totals and unique cloners together, selectable metric statistics, scoped ranking/share, and complete JSONL export.
- Every development port is loopback-only, and no secret is tracked in source control.
