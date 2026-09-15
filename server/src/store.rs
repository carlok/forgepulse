use std::{collections::BTreeMap, str::FromStr};

use anyhow::Context;
use chrono::{Duration, NaiveDate, Utc};
use sqlx::{
    SqlitePool,
    sqlite::{SqliteConnectOptions, SqliteJournalMode},
};

use crate::{
    models::{
        CloneChartPoint, DayPoint, Diagnosis, HumanAttention, HumanAttentionComponents, PathPoint,
        RankTrend, ReferrerPoint, Repository, RepositorySummary, StarPoint, SyncRun,
    },
    stats,
};

#[derive(Clone)]
pub struct Store {
    pool: SqlitePool,
}

impl Store {
    pub async fn connect(database_url: &str) -> anyhow::Result<Self> {
        let options = SqliteConnectOptions::from_str(database_url)
            .context("parse SQLite database URL")?
            .create_if_missing(true)
            .foreign_keys(true)
            .journal_mode(SqliteJournalMode::Wal)
            .busy_timeout(std::time::Duration::from_secs(5));
        let pool = SqlitePool::connect_with(options).await?;
        sqlx::migrate!("./migrations").run(&pool).await?;
        Ok(Self { pool })
    }

    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    pub async fn upsert_repository(&self, repository: &Repository) -> anyhow::Result<()> {
        sqlx::query(
            r#"INSERT INTO repositories
              (name, description, stars, forks, watchers, issues, pull_requests, is_fork, is_archived, created_at, updated_at)
              VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
              ON CONFLICT(name) DO UPDATE SET
                description=excluded.description, stars=excluded.stars, forks=excluded.forks,
                watchers=excluded.watchers, issues=excluded.issues, pull_requests=excluded.pull_requests,
                is_fork=excluded.is_fork, is_archived=excluded.is_archived,
                created_at=excluded.created_at, updated_at=excluded.updated_at"#,
        )
        .bind(&repository.name)
        .bind(&repository.description)
        .bind(repository.stars)
        .bind(repository.forks)
        .bind(repository.watchers)
        .bind(repository.issues)
        .bind(repository.pull_requests)
        .bind(repository.is_fork)
        .bind(repository.is_archived)
        .bind(&repository.created_at)
        .bind(&repository.updated_at)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn upsert_daily_traffic(
        &self,
        repository_name: &str,
        day: &str,
        metric: &str,
        count: i64,
        uniques: i64,
    ) -> anyhow::Result<()> {
        sqlx::query(
            r#"INSERT INTO daily_traffic (repository_name, day, metric, count, uniques)
               VALUES (?, ?, ?, ?, ?)
               ON CONFLICT(repository_name, day, metric) DO UPDATE SET
                 count=excluded.count, uniques=excluded.uniques"#,
        )
        .bind(repository_name)
        .bind(day)
        .bind(metric)
        .bind(count)
        .bind(uniques)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn upsert_referrer(
        &self,
        repository_name: &str,
        captured_on: &str,
        referrer: &str,
        count: i64,
        uniques: i64,
    ) -> anyhow::Result<()> {
        sqlx::query(
            r#"INSERT INTO referrer_snapshots (repository_name, captured_on, referrer, count, uniques)
               VALUES (?, ?, ?, ?, ?)
               ON CONFLICT(repository_name, captured_on, referrer) DO UPDATE SET
                 count=excluded.count, uniques=excluded.uniques"#,
        )
        .bind(repository_name)
        .bind(captured_on)
        .bind(referrer)
        .bind(count)
        .bind(uniques)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn upsert_path(
        &self,
        repository_name: &str,
        captured_on: &str,
        path: &str,
        title: &str,
        count: i64,
        uniques: i64,
    ) -> anyhow::Result<()> {
        sqlx::query(
            r#"INSERT INTO path_snapshots (repository_name, captured_on, path, title, count, uniques)
               VALUES (?, ?, ?, ?, ?, ?)
               ON CONFLICT(repository_name, captured_on, path) DO UPDATE SET
                 title=excluded.title, count=excluded.count, uniques=excluded.uniques"#,
        )
        .bind(repository_name)
        .bind(captured_on)
        .bind(path)
        .bind(title)
        .bind(count)
        .bind(uniques)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn upsert_star(
        &self,
        repository_name: &str,
        day: &str,
        total: i64,
    ) -> anyhow::Result<()> {
        sqlx::query(
            r#"INSERT INTO star_history (repository_name, day, total) VALUES (?, ?, ?)
               ON CONFLICT(repository_name, day) DO UPDATE SET total=excluded.total"#,
        )
        .bind(repository_name)
        .bind(day)
        .bind(total)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn upsert_fork(
        &self,
        repository_name: &str,
        day: &str,
        total: i64,
    ) -> anyhow::Result<()> {
        sqlx::query(
            r#"INSERT INTO fork_history (repository_name, day, total) VALUES (?, ?, ?)
               ON CONFLICT(repository_name, day) DO UPDATE SET total=excluded.total"#,
        )
        .bind(repository_name)
        .bind(day)
        .bind(total)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn upsert_release_event(
        &self,
        repository_name: &str,
        kind: &str,
        name: &str,
        occurred_on: &str,
    ) -> anyhow::Result<()> {
        sqlx::query(
            r#"INSERT INTO release_events (repository_name, kind, name, occurred_on) VALUES (?, ?, ?, ?)
               ON CONFLICT(repository_name, kind, name) DO UPDATE SET occurred_on=excluded.occurred_on"#,
        )
        .bind(repository_name).bind(kind).bind(name).bind(occurred_on).execute(&self.pool).await?;
        Ok(())
    }

    pub async fn upsert_actions_checkout_estimate(
        &self,
        repository_name: &str,
        day: &str,
        completed_jobs: i64,
    ) -> anyhow::Result<()> {
        sqlx::query(
            r#"INSERT INTO actions_checkout_snapshots (repository_name, captured_on, completed_checkout_jobs) VALUES (?, ?, ?)
               ON CONFLICT(repository_name, captured_on) DO UPDATE SET completed_checkout_jobs=excluded.completed_checkout_jobs"#,
        )
        .bind(repository_name).bind(day).bind(completed_jobs).execute(&self.pool).await?;
        Ok(())
    }

    pub async fn start_sync_run(&self) -> anyhow::Result<i64> {
        let result =
            sqlx::query("INSERT INTO sync_runs (started_at, status) VALUES (?, 'running')")
                .bind(Utc::now().to_rfc3339())
                .execute(&self.pool)
                .await?;
        Ok(result.last_insert_rowid())
    }

    pub async fn finish_sync_run(
        &self,
        id: i64,
        status: &str,
        repositories_synced: i64,
        message: Option<&str>,
    ) -> anyhow::Result<()> {
        sqlx::query("UPDATE sync_runs SET finished_at=?, status=?, repositories_synced=?, message=? WHERE id=?")
            .bind(Utc::now().to_rfc3339())
            .bind(status)
            .bind(repositories_synced)
            .bind(message)
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn sync_runs(&self) -> anyhow::Result<Vec<SyncRun>> {
        Ok(sqlx::query_as("SELECT id, started_at, finished_at, status, repositories_synced, message FROM sync_runs ORDER BY id DESC LIMIT 20")
            .fetch_all(&self.pool)
            .await?)
    }

    pub async fn repository_summaries(
        &self,
        search: &str,
    ) -> anyhow::Result<Vec<RepositorySummary>> {
        let rows = sqlx::query_as::<_, SummaryRow>(
            r#"SELECT r.name, r.description, r.stars, r.forks, r.watchers, r.issues, r.pull_requests,
                 r.is_fork, r.is_archived, r.created_at, r.updated_at,
                 COALESCE(SUM(CASE WHEN d.metric='view' THEN d.count END), 0) AS total_views,
                 COALESCE(SUM(CASE WHEN d.metric='view' THEN d.uniques END), 0) AS total_view_uniques,
                 COALESCE(SUM(CASE WHEN d.metric='clone' THEN d.count END), 0) AS total_clones,
                 COALESCE(SUM(CASE WHEN d.metric='clone' THEN d.uniques END), 0) AS total_clone_uniques,
                 COALESCE(SUM(CASE WHEN d.metric='clone' AND d.day >= date('now', '-1 day') THEN d.count END), 0) AS clones_1d,
                 COALESCE(SUM(CASE WHEN d.metric='clone' AND d.day >= date('now', '-6 day') THEN d.count END), 0) AS clones_7d,
                 COALESCE(SUM(CASE WHEN d.metric='clone' AND d.day >= date('now', '-29 day') THEN d.count END), 0) AS clones_30d,
                 COALESCE(SUM(CASE WHEN d.metric='clone' AND d.day >= date('now', '-29 day') THEN d.uniques END), 0) AS unique_cloners_30d,
                 COALESCE(SUM(CASE WHEN d.metric='view' AND d.day >= date('now', '-29 day') THEN d.count END), 0) AS views_30d,
                 COALESCE(SUM(CASE WHEN d.metric='view' AND d.day >= date('now', '-6 day') THEN d.uniques END), 0) AS unique_views_7d
               FROM repositories r LEFT JOIN daily_traffic d ON d.repository_name=r.name
               WHERE lower(r.name) LIKE lower(?)
               GROUP BY r.name ORDER BY total_clones DESC, r.name ASC"#,
        )
        .bind(format!("%{}%", search.trim()))
        .fetch_all(&self.pool)
        .await?;
        let names = rows.iter().map(|row| row.name.as_str()).collect::<Vec<_>>();
        let daily_medians = self.repository_clone_daily_medians(&names).await?;
        let total_clones = rows.iter().map(|row| row.total_clones).sum::<i64>();
        let clone_ranks = dense_rank_by_total(
            rows.iter()
                .map(|row| (row.name.clone(), row.total_clones))
                .collect(),
        );
        let mut summaries = Vec::with_capacity(rows.len());
        for row in rows {
            let rank = clone_ranks[&row.name];
            let clone_daily_median = daily_medians.get(&row.name).copied();
            let name = row.name.clone();
            let created_at = row.created_at.clone();
            let human_attention = self.human_attention(&name, row.unique_views_7d).await?;
            let diagnoses = self
                .diagnoses(
                    &name,
                    row.clones_1d,
                    row.clones_30d,
                    row.unique_cloners_30d,
                    row.views_30d,
                    &created_at,
                )
                .await?;
            summaries.push(RepositorySummary {
                repository: Repository {
                    name,
                    description: row.description,
                    stars: row.stars,
                    forks: row.forks,
                    watchers: row.watchers,
                    issues: row.issues,
                    pull_requests: row.pull_requests,
                    is_fork: row.is_fork,
                    is_archived: row.is_archived,
                    created_at,
                    updated_at: row.updated_at,
                },
                total_views: row.total_views,
                total_view_uniques: row.total_view_uniques,
                total_clones: row.total_clones,
                total_clone_uniques: row.total_clone_uniques,
                clones_1d: row.clones_1d,
                clones_7d: row.clones_7d,
                clones_30d: row.clones_30d,
                clone_rank: rank,
                clone_rank_trend: RankTrend::Unknown,
                clone_share_percent: if total_clones == 0 {
                    0.0
                } else {
                    row.total_clones as f64 * 100.0 / total_clones as f64
                },
                clone_daily_median,
                human_attention,
                attention_rank_trend: RankTrend::Unknown,
                diagnoses,
            });
        }
        assign_human_attention_ranks(&mut summaries);
        self.apply_rank_trends(&mut summaries, search).await?;
        Ok(summaries)
    }

    /// Median of each repository's own daily clone counts (zero-filled), keyed by repository
    /// name — the per-repo counterpart to the fleet-wide median in `clone_chart`'s statistics,
    /// so the two are comparable on the same daily-count basis.
    async fn repository_clone_daily_medians(
        &self,
        names: &[&str],
    ) -> anyhow::Result<BTreeMap<String, f64>> {
        if names.is_empty() {
            return Ok(BTreeMap::new());
        }
        let mut builder = sqlx::QueryBuilder::new(
            "SELECT repository_name, day, count, uniques FROM daily_traffic WHERE metric='clone' AND repository_name IN (",
        );
        {
            let mut separated = builder.separated(", ");
            for name in names {
                separated.push_bind(*name);
            }
        }
        builder.push(") ORDER BY repository_name, day");
        let rows = builder
            .build_query_as::<RepoDayRow>()
            .fetch_all(&self.pool)
            .await?;

        let mut by_repo: BTreeMap<String, Vec<DayPoint>> = BTreeMap::new();
        for row in rows {
            by_repo
                .entry(row.repository_name)
                .or_default()
                .push(DayPoint {
                    day: row.day,
                    count: row.count,
                    uniques: row.uniques,
                });
        }

        Ok(by_repo
            .into_iter()
            .filter_map(|(name, points)| {
                let counts = zero_fill(points)
                    .into_iter()
                    .map(|point| point.count)
                    .collect::<Vec<_>>();
                stats::clone_statistics(&counts).map(|stats| (name, stats.median))
            })
            .collect())
    }

    pub async fn clone_chart(
        &self,
        repositories: &[RepositorySummary],
    ) -> anyhow::Result<Vec<CloneChartPoint>> {
        Ok(self
            .traffic_chart(repositories, "clone")
            .await?
            .into_iter()
            .map(|point| CloneChartPoint {
                day: point.day,
                total_clones: point.count,
                unique_cloners: point.uniques,
            })
            .collect())
    }

    pub async fn views_chart(
        &self,
        repositories: &[RepositorySummary],
    ) -> anyhow::Result<Vec<DayPoint>> {
        self.traffic_chart(repositories, "view").await
    }

    async fn traffic_chart(
        &self,
        repositories: &[RepositorySummary],
        metric: &str,
    ) -> anyhow::Result<Vec<DayPoint>> {
        if repositories.is_empty() {
            return Ok(Vec::new());
        }
        let names = repositories
            .iter()
            .map(|repo| repo.repository.name.as_str())
            .collect::<Vec<_>>();
        let mut builder = sqlx::QueryBuilder::new(
            "SELECT day, SUM(count) AS count, SUM(uniques) AS uniques FROM daily_traffic WHERE metric=",
        );
        builder.push_bind(metric);
        builder.push(" AND repository_name IN (");
        {
            let mut separated = builder.separated(", ");
            for name in names {
                separated.push_bind(name);
            }
        }
        builder.push(") GROUP BY day ORDER BY day ASC");
        let points = builder
            .build_query_as::<DayPoint>()
            .fetch_all(&self.pool)
            .await?;
        Ok(zero_fill(points))
    }

    pub async fn traffic(
        &self,
        repository_name: &str,
        metric: &str,
    ) -> anyhow::Result<Vec<DayPoint>> {
        Ok(sqlx::query_as("SELECT day, count, uniques FROM daily_traffic WHERE repository_name=? AND metric=? ORDER BY day")
            .bind(repository_name)
            .bind(metric)
            .fetch_all(&self.pool)
            .await?)
    }

    pub async fn referrers(&self, repository_name: &str) -> anyhow::Result<Vec<ReferrerPoint>> {
        Ok(sqlx::query_as("SELECT captured_on, referrer, count, uniques FROM referrer_snapshots WHERE repository_name=? ORDER BY captured_on DESC, count DESC")
            .bind(repository_name).fetch_all(&self.pool).await?)
    }

    /// The most recently captured referrer snapshot only, for display — `referrers` keeps the
    /// full daily history (used by the JSONL export), but GitHub's referrer/path stats rarely
    /// change day to day, so showing every stored day in the UI reads as duplicate rows.
    pub async fn latest_referrers(
        &self,
        repository_name: &str,
    ) -> anyhow::Result<Vec<ReferrerPoint>> {
        Ok(sqlx::query_as(
            r#"SELECT captured_on, referrer, count, uniques FROM referrer_snapshots
               WHERE repository_name = ? AND captured_on = (
                 SELECT MAX(captured_on) FROM referrer_snapshots WHERE repository_name = ?
               )
               ORDER BY count DESC"#,
        )
        .bind(repository_name)
        .bind(repository_name)
        .fetch_all(&self.pool)
        .await?)
    }

    pub async fn paths(&self, repository_name: &str) -> anyhow::Result<Vec<PathPoint>> {
        Ok(sqlx::query_as("SELECT captured_on, path, title, count, uniques FROM path_snapshots WHERE repository_name=? ORDER BY captured_on DESC, count DESC")
            .bind(repository_name).fetch_all(&self.pool).await?)
    }

    /// The most recently captured path snapshot only — see `latest_referrers`.
    pub async fn latest_paths(&self, repository_name: &str) -> anyhow::Result<Vec<PathPoint>> {
        Ok(sqlx::query_as(
            r#"SELECT captured_on, path, title, count, uniques FROM path_snapshots
               WHERE repository_name = ? AND captured_on = (
                 SELECT MAX(captured_on) FROM path_snapshots WHERE repository_name = ?
               )
               ORDER BY count DESC"#,
        )
        .bind(repository_name)
        .bind(repository_name)
        .fetch_all(&self.pool)
        .await?)
    }

    pub async fn stars(&self, repository_name: &str) -> anyhow::Result<Vec<StarPoint>> {
        Ok(sqlx::query_as(
            "SELECT day, total FROM star_history WHERE repository_name=? ORDER BY day",
        )
        .bind(repository_name)
        .fetch_all(&self.pool)
        .await?)
    }

    pub async fn forks(&self, repository_name: &str) -> anyhow::Result<Vec<StarPoint>> {
        Ok(sqlx::query_as(
            "SELECT day, total FROM fork_history WHERE repository_name=? ORDER BY day",
        )
        .bind(repository_name)
        .fetch_all(&self.pool)
        .await?)
    }

    async fn human_attention(
        &self,
        repository_name: &str,
        unique_views_7d: i64,
    ) -> anyhow::Result<HumanAttention> {
        let today = Utc::now().date_naive().to_string();
        let (external_referrer_uniques_14d, new_stars_30d, new_forks_30d) = self
            .attention_components_as_of(repository_name, &today)
            .await?;
        let components = HumanAttentionComponents {
            unique_views_7d: Some(unique_views_7d),
            external_referrer_uniques_14d,
            new_stars_30d,
            new_forks_30d,
        };
        let score = attention_score(
            unique_views_7d,
            external_referrer_uniques_14d,
            new_stars_30d,
            new_forks_30d,
        );
        Ok(HumanAttention {
            version: "v1".into(),
            score,
            rank: None,
            components,
        })
    }

    /// The 3 inputs to [`attention_score`] besides `unique_views_7d`, as they stood on `as_of`
    /// (a `YYYY-MM-DD` date) — re-runnable for a past date since the underlying snapshot/history
    /// tables are retained and day-keyed, which is what lets [`Self::apply_rank_trends`] compare
    /// today's ranking to yesterday's without a dedicated history-of-ranks table.
    async fn attention_components_as_of(
        &self,
        repository_name: &str,
        as_of: &str,
    ) -> anyhow::Result<(Option<i64>, Option<i64>, Option<i64>)> {
        let latest_referrer_snapshot: Option<String> = sqlx::query_scalar(
            "SELECT MAX(captured_on) FROM referrer_snapshots WHERE repository_name=? AND captured_on<=?",
        )
        .bind(repository_name)
        .bind(as_of)
        .fetch_one(&self.pool)
        .await?;
        let external_referrer_uniques_14d = match latest_referrer_snapshot {
            Some(captured_on) => Some(sqlx::query_scalar(
                r#"SELECT COALESCE(SUM(uniques), 0) FROM referrer_snapshots
                   WHERE repository_name=? AND captured_on=?
                     AND lower(referrer) != 'github.com' AND lower(referrer) NOT LIKE '%.github.com'"#,
            ).bind(repository_name).bind(captured_on).fetch_one(&self.pool).await?),
            None => None,
        };
        let stars = self
            .history_delta_as_of("star_history", repository_name, as_of)
            .await?;
        let forks = self
            .history_delta_as_of("fork_history", repository_name, as_of)
            .await?;
        Ok((external_referrer_uniques_14d, stars, forks))
    }

    async fn history_delta_as_of(
        &self,
        table: &str,
        repository_name: &str,
        as_of: &str,
    ) -> anyhow::Result<Option<i64>> {
        let cutoff = (NaiveDate::parse_from_str(as_of, "%Y-%m-%d").context("parse as_of date")?
            - Duration::days(30))
        .to_string();
        let query = format!("SELECT total FROM {table} WHERE repository_name=? AND day=?");
        let Some(baseline) = sqlx::query_scalar::<_, i64>(&query)
            .bind(repository_name)
            .bind(&cutoff)
            .fetch_optional(&self.pool)
            .await?
        else {
            return Ok(None);
        };
        // Bounded by `as_of`, not just "the latest row ever" — required once `as_of` can be
        // "yesterday", or a row inserted for today would leak into yesterday's delta.
        let query = format!(
            "SELECT total FROM {table} WHERE repository_name=? AND day<=? ORDER BY day DESC LIMIT 1"
        );
        let latest = sqlx::query_scalar::<_, i64>(&query)
            .bind(repository_name)
            .bind(as_of)
            .fetch_optional(&self.pool)
            .await?;
        Ok(latest.map(|total| (total - baseline).max(0)))
    }

    async fn diagnoses(
        &self,
        repository_name: &str,
        clones_1d: i64,
        clones_30d: i64,
        unique_cloners: i64,
        views: i64,
        created_at: &str,
    ) -> anyhow::Result<Vec<Diagnosis>> {
        let mut diagnoses = Vec::new();
        let clone_burst = clones_30d >= 50
            && views > 0
            && clones_30d >= views * 4
            && unique_cloners > 0
            && clones_30d >= unique_cloners * 3
            && clones_1d * 100 >= clones_30d * 80;
        if clone_burst {
            diagnoses.push(Diagnosis {
                kind: "clone_burst".into(),
                evidence: vec![
                    format!("{clones_30d} clones in 30d"),
                    format!("{views} views"),
                    format!("{unique_cloners} unique cloners"),
                    format!("{clones_1d} clones in the latest day"),
                ],
            });
        }
        let cutoff = (Utc::now().date_naive() - Duration::days(30)).to_string();
        let recent_event: Option<String> = sqlx::query_scalar(
            "SELECT kind || ' ' || name FROM release_events WHERE repository_name=? AND occurred_on >= ? ORDER BY occurred_on DESC LIMIT 1",
        ).bind(repository_name).bind(&cutoff).fetch_optional(&self.pool).await?;
        let recent_repository = NaiveDate::parse_from_str(created_at, "%Y-%m-%d")
            .ok()
            .is_some_and(|day| day >= Utc::now().date_naive() - Duration::days(7));
        if recent_repository || recent_event.is_some() {
            let evidence =
                recent_event.unwrap_or_else(|| format!("repository created {created_at}"));
            diagnoses.push(Diagnosis {
                kind: "launch_shaped".into(),
                evidence: vec![evidence],
            });
        }
        let checkout_jobs: Option<i64> = sqlx::query_scalar(
            "SELECT completed_checkout_jobs FROM actions_checkout_snapshots WHERE repository_name=? ORDER BY captured_on DESC LIMIT 1",
        ).bind(repository_name).fetch_optional(&self.pool).await?;
        if clone_burst && checkout_jobs.is_some_and(|jobs| jobs >= 50) {
            diagnoses.push(Diagnosis {
                kind: "automation_likely".into(),
                evidence: vec![format!(
                    "estimated {} completed jobs in workflows containing actions/checkout",
                    checkout_jobs.unwrap_or_default()
                )],
            });
        }
        Ok(diagnoses)
    }

    pub async fn seed_demo(&self) -> anyhow::Result<()> {
        let today = Utc::now().date_naive();
        for (name, stars, base) in [("carlok/forgepulse", 12, 20), ("carlok/metrics-lab", 4, 7)] {
            self.upsert_repository(&Repository {
                name: name.to_string(),
                description: "Demo repository".to_string(),
                stars,
                forks: 1,
                watchers: 1,
                issues: 0,
                pull_requests: 0,
                is_fork: false,
                is_archived: false,
                created_at: today.to_string(),
                updated_at: today.to_string(),
            })
            .await?;
            for offset in 0..14 {
                let day = today - Duration::days(13 - offset);
                let count = base + offset * 2;
                self.upsert_daily_traffic(name, &day.to_string(), "clone", count, count / 2)
                    .await?;
                self.upsert_daily_traffic(name, &day.to_string(), "view", count * 3, count)
                    .await?;
            }
            self.upsert_star(name, &today.to_string(), stars).await?;
            self.upsert_fork(name, &today.to_string(), 1).await?;
        }
        Ok(())
    }

    async fn clone_totals_as_of(
        &self,
        search: &str,
        as_of: &str,
    ) -> anyhow::Result<BTreeMap<String, i64>> {
        let rows: Vec<(String, i64)> = sqlx::query_as(
            r#"SELECT r.name,
                 COALESCE(SUM(CASE WHEN d.metric='clone' AND d.day<=? THEN d.count END), 0) AS total_clones
               FROM repositories r LEFT JOIN daily_traffic d ON d.repository_name=r.name
               WHERE lower(r.name) LIKE lower(?)
               GROUP BY r.name"#,
        )
        .bind(as_of)
        .bind(format!("%{}%", search.trim()))
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().collect())
    }

    async fn attention_scores_as_of(
        &self,
        search: &str,
        as_of: &str,
    ) -> anyhow::Result<BTreeMap<String, Option<f64>>> {
        let rows: Vec<(String, i64)> = sqlx::query_as(
            r#"SELECT r.name,
                 COALESCE(SUM(CASE WHEN d.metric='view' AND d.day<=? AND d.day>=date(?, '-6 day') THEN d.uniques END), 0) AS unique_views_7d
               FROM repositories r LEFT JOIN daily_traffic d ON d.repository_name=r.name
               WHERE lower(r.name) LIKE lower(?)
               GROUP BY r.name"#,
        )
        .bind(as_of)
        .bind(as_of)
        .bind(format!("%{}%", search.trim()))
        .fetch_all(&self.pool)
        .await?;
        let mut scores = BTreeMap::new();
        for (name, unique_views_7d) in rows {
            let (referrers, stars, forks) = self.attention_components_as_of(&name, as_of).await?;
            scores.insert(
                name,
                attention_score(unique_views_7d, referrers, stars, forks),
            );
        }
        Ok(scores)
    }

    /// Sets `clone_rank_trend`/`attention_rank_trend` on every summary by comparing today's
    /// rank (already computed) to a from-scratch re-ranking as of yesterday, scoped by the same
    /// `search` filter so the two rankings are over the same set of repositories.
    async fn apply_rank_trends(
        &self,
        summaries: &mut [RepositorySummary],
        search: &str,
    ) -> anyhow::Result<()> {
        let yesterday = (Utc::now().date_naive() - Duration::days(1)).to_string();
        let yesterday_clone_ranks =
            dense_rank_by_total(self.clone_totals_as_of(search, &yesterday).await?);
        let yesterday_attention_ranks =
            dense_rank_by_score(self.attention_scores_as_of(search, &yesterday).await?);

        for summary in summaries.iter_mut() {
            let name = summary.repository.name.clone();
            let existed = existed_as_of(&summary.repository.created_at, &yesterday);

            summary.clone_rank_trend = match (existed, yesterday_clone_ranks.get(&name)) {
                (true, Some(&yesterday_rank)) => compare_rank(summary.clone_rank, yesterday_rank),
                _ => RankTrend::Unknown,
            };

            summary.attention_rank_trend = match (
                existed,
                summary.human_attention.rank,
                yesterday_attention_ranks.get(&name),
            ) {
                (true, Some(today_rank), Some(&yesterday_rank)) => {
                    compare_rank(today_rank, yesterday_rank)
                }
                _ => RankTrend::Unknown,
            };
        }
        Ok(())
    }
}

#[derive(sqlx::FromRow)]
struct SummaryRow {
    name: String,
    description: String,
    stars: i64,
    forks: i64,
    watchers: i64,
    issues: i64,
    pull_requests: i64,
    is_fork: bool,
    is_archived: bool,
    created_at: String,
    updated_at: String,
    total_views: i64,
    total_view_uniques: i64,
    total_clones: i64,
    total_clone_uniques: i64,
    clones_1d: i64,
    clones_7d: i64,
    clones_30d: i64,
    unique_cloners_30d: i64,
    views_30d: i64,
    unique_views_7d: i64,
}

#[derive(sqlx::FromRow)]
struct RepoDayRow {
    repository_name: String,
    day: String,
    count: i64,
    uniques: i64,
}

fn zero_fill(points: Vec<DayPoint>) -> Vec<DayPoint> {
    let Some(first) = points.first() else {
        return Vec::new();
    };
    let last = points.last().expect("first point has last point");
    let start = NaiveDate::parse_from_str(&first.day, "%Y-%m-%d").expect("stored date is valid");
    let end = NaiveDate::parse_from_str(&last.day, "%Y-%m-%d").expect("stored date is valid");
    let by_day = points
        .into_iter()
        .map(|point| (point.day, (point.count, point.uniques)))
        .collect::<BTreeMap<_, _>>();
    let start = std::cmp::max(start, end - Duration::days(119));
    let mut filled = Vec::new();
    let mut day = start;
    while day <= end {
        let key = day.to_string();
        let (count, uniques) = by_day.get(&key).copied().unwrap_or_default();
        filled.push(DayPoint {
            day: key,
            count,
            uniques,
        });
        day += Duration::days(1);
    }
    filled
}

fn assign_human_attention_ranks(summaries: &mut [RepositorySummary]) {
    let scores = summaries
        .iter()
        .map(|summary| {
            (
                summary.repository.name.clone(),
                summary.human_attention.score,
            )
        })
        .collect();
    let ranks = dense_rank_by_score(scores);
    for summary in summaries.iter_mut() {
        summary.human_attention.rank = ranks.get(&summary.repository.name).copied();
    }
}

/// Same ln_1p-weighted formula `human_attention` has always used — extracted so it isn't
/// duplicated between "as of today" and "as of yesterday" (`attention_scores_as_of`).
fn attention_score(
    unique_views_7d: i64,
    external_referrer_uniques_14d: Option<i64>,
    new_stars_30d: Option<i64>,
    new_forks_30d: Option<i64>,
) -> Option<f64> {
    match (external_referrer_uniques_14d, new_stars_30d, new_forks_30d) {
        (Some(referrers), Some(stars), Some(forks)) => Some(
            (unique_views_7d as f64).ln_1p()
                + 2.0 * (referrers as f64).ln_1p()
                + 4.0 * (stars as f64).ln_1p()
                + 6.0 * (forks as f64).ln_1p(),
        ),
        _ => None,
    }
}

/// Dense rank (ties share a rank, next rank skips ahead) descending by total, `name` breaking
/// ties — the same tiebreak `repository_summaries`'s SQL `ORDER BY` already used, now shared so
/// today's and yesterday's rankings break ties identically (otherwise a tie could show a
/// spurious up/down purely from ordering, not a real change).
fn dense_rank_by_total(totals: BTreeMap<String, i64>) -> BTreeMap<String, usize> {
    let mut ordered = totals.into_iter().collect::<Vec<_>>();
    ordered.sort_by(|(left_name, left_total), (right_name, right_total)| {
        right_total
            .cmp(left_total)
            .then_with(|| left_name.cmp(right_name))
    });
    let mut ranks = BTreeMap::new();
    let mut previous = None;
    let mut rank = 0;
    for (index, (name, total)) in ordered.into_iter().enumerate() {
        if previous != Some(total) {
            rank = index + 1;
            previous = Some(total);
        }
        ranks.insert(name, rank);
    }
    ranks
}

/// Same dense-rank shape as [`dense_rank_by_total`], but descending by an optional score —
/// repositories with no score (`None`) are left unranked rather than sorted to one end.
fn dense_rank_by_score(scores: BTreeMap<String, Option<f64>>) -> BTreeMap<String, usize> {
    let mut ordered = scores
        .into_iter()
        .filter_map(|(name, score)| score.map(|score| (name, score)))
        .collect::<Vec<_>>();
    ordered.sort_by(|(left_name, left_score), (right_name, right_score)| {
        right_score
            .partial_cmp(left_score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| left_name.cmp(right_name))
    });
    let mut ranks = BTreeMap::new();
    let mut previous = None;
    let mut rank = 0;
    for (index, (name, score)) in ordered.into_iter().enumerate() {
        if previous != Some(score) {
            rank = index + 1;
            previous = Some(score);
        }
        ranks.insert(name, rank);
    }
    ranks
}

/// Whether a repository's stored `created_at` (a `YYYY-MM-DD` date, possibly empty/unparseable
/// for older rows) is on or before `as_of` — an unparseable date is treated as "existed", the
/// same lenient convention `diagnoses`'s recency check already uses for this same column.
fn existed_as_of(created_at: &str, as_of: &str) -> bool {
    NaiveDate::parse_from_str(created_at, "%Y-%m-%d")
        .ok()
        .is_none_or(|day| day.to_string().as_str() <= as_of)
}

fn compare_rank(today: usize, yesterday: usize) -> RankTrend {
    match today.cmp(&yesterday) {
        std::cmp::Ordering::Less => RankTrend::Up,
        std::cmp::Ordering::Greater => RankTrend::Down,
        std::cmp::Ordering::Equal => RankTrend::Stable,
    }
}

pub fn statistics_for_chart(
    chart: &[CloneChartPoint],
) -> (
    Option<crate::models::CloneStatistics>,
    Option<crate::models::CloneStatistics>,
) {
    (
        stats::total_clone_statistics(chart),
        stats::unique_clone_statistics(chart),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn human_attention_fixture_catalog_covers_requested_cases() {
        let cases: Vec<serde_json::Value> =
            serde_json::from_str(include_str!("../tests/fixtures/human_attention_cases.json"))
                .expect("fixture JSON");
        assert_eq!(cases.len(), 5);
        assert!(
            cases
                .iter()
                .any(|case| case["name"] == "missing-star-fork-history")
        );
    }

    #[tokio::test]
    async fn human_attention_is_explainable_and_diagnoses_clone_bursts() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let store = Store::connect(&format!(
            "sqlite:{}",
            directory.path().join("traffic.db").display()
        ))
        .await
        .expect("store");
        let today = Utc::now().date_naive();
        let name = "carlok/ci-heavy";
        store
            .upsert_repository(&Repository {
                name: name.into(),
                description: String::new(),
                stars: 15,
                forks: 3,
                watchers: 0,
                issues: 0,
                pull_requests: 0,
                is_fork: false,
                is_archived: false,
                created_at: (today - Duration::days(2)).to_string(),
                updated_at: today.to_string(),
            })
            .await
            .expect("repository");
        store
            .upsert_daily_traffic(name, &today.to_string(), "clone", 100, 20)
            .await
            .expect("clones");
        store
            .upsert_daily_traffic(name, &today.to_string(), "view", 10, 8)
            .await
            .expect("views");
        store
            .upsert_referrer(name, &today.to_string(), "example.com", 12, 7)
            .await
            .expect("referrer");
        store
            .upsert_star(name, &(today - Duration::days(30)).to_string(), 2)
            .await
            .expect("star baseline");
        store
            .upsert_star(name, &today.to_string(), 15)
            .await
            .expect("star latest");
        store
            .upsert_fork(name, &(today - Duration::days(30)).to_string(), 1)
            .await
            .expect("fork baseline");
        store
            .upsert_fork(name, &today.to_string(), 3)
            .await
            .expect("fork latest");
        store
            .upsert_actions_checkout_estimate(name, &today.to_string(), 80)
            .await
            .expect("actions");
        let mut summaries = store.repository_summaries(name).await.expect("summary");
        let summary = summaries.remove(0);
        assert_eq!(summary.human_attention.version, "v1");
        assert_eq!(summary.human_attention.components.new_stars_30d, Some(13));
        assert_eq!(summary.human_attention.components.new_forks_30d, Some(2));
        assert!(summary.human_attention.score.is_some());
        assert!(
            summary
                .diagnoses
                .iter()
                .any(|diagnosis| diagnosis.kind == "clone_burst")
        );
        assert!(
            summary
                .diagnoses
                .iter()
                .any(|diagnosis| diagnosis.kind == "launch_shaped")
        );
        assert!(
            summary
                .diagnoses
                .iter()
                .any(|diagnosis| diagnosis.kind == "automation_likely")
        );
    }

    #[test]
    fn chart_fills_missing_calendar_days() {
        let chart = zero_fill(vec![
            DayPoint {
                day: "2026-01-01".into(),
                count: 3,
                uniques: 2,
            },
            DayPoint {
                day: "2026-01-03".into(),
                count: 5,
                uniques: 4,
            },
        ]);
        assert_eq!(chart.len(), 3);
        assert_eq!(chart[1].count, 0);
    }

    #[tokio::test]
    async fn clone_daily_median_is_computed_over_zero_filled_days() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let database = format!("sqlite:{}", directory.path().join("traffic.db").display());
        let store = Store::connect(&database).await.expect("connect");
        store
            .upsert_repository(&Repository {
                name: "carlok/alpha".into(),
                description: String::new(),
                stars: 0,
                forks: 0,
                watchers: 0,
                issues: 0,
                pull_requests: 0,
                is_fork: false,
                is_archived: false,
                created_at: "2026-08-01".into(),
                updated_at: "2026-08-01".into(),
            })
            .await
            .expect("repository");
        // 2026-08-02 is deliberately skipped: zero_fill must count it as 0, otherwise the
        // median (2, 0, 8 -> 2) would come out as the wrong-but-plausible (2, 8 -> 5).
        store
            .upsert_daily_traffic("carlok/alpha", "2026-08-01", "clone", 2, 1)
            .await
            .expect("traffic");
        store
            .upsert_daily_traffic("carlok/alpha", "2026-08-03", "clone", 8, 3)
            .await
            .expect("traffic");

        let summaries = store.repository_summaries("").await.expect("summaries");
        assert_eq!(summaries.len(), 1);
        assert_eq!(summaries[0].clone_daily_median, Some(2.0));
    }

    #[tokio::test]
    async fn clone_daily_median_is_none_for_a_repository_with_no_clone_history() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let database = format!("sqlite:{}", directory.path().join("traffic.db").display());
        let store = Store::connect(&database).await.expect("connect");
        store
            .upsert_repository(&Repository {
                name: "carlok/alpha".into(),
                description: String::new(),
                stars: 0,
                forks: 0,
                watchers: 0,
                issues: 0,
                pull_requests: 0,
                is_fork: false,
                is_archived: false,
                created_at: "2026-08-01".into(),
                updated_at: "2026-08-01".into(),
            })
            .await
            .expect("repository");

        let summaries = store.repository_summaries("").await.expect("summaries");
        assert_eq!(summaries.len(), 1);
        assert_eq!(summaries[0].clone_daily_median, None);
    }

    #[tokio::test]
    async fn latest_referrers_and_paths_skip_older_identical_snapshots() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let database = format!("sqlite:{}", directory.path().join("traffic.db").display());
        let store = Store::connect(&database).await.expect("connect");
        store
            .upsert_repository(&Repository {
                name: "carlok/alpha".into(),
                description: String::new(),
                stars: 0,
                forks: 0,
                watchers: 0,
                issues: 0,
                pull_requests: 0,
                is_fork: false,
                is_archived: false,
                created_at: "2026-08-01".into(),
                updated_at: "2026-08-01".into(),
            })
            .await
            .expect("repository");
        // GitHub's referrer/path stats often don't change day to day, so the same values get
        // captured on consecutive days — `latest_*` must still return each list only once.
        for day in ["2026-08-23", "2026-08-24"] {
            store
                .upsert_referrer("carlok/alpha", day, "github.com", 310, 7)
                .await
                .expect("referrer");
            store
                .upsert_path("carlok/alpha", day, "/carlok/alpha", "Overview", 190, 18)
                .await
                .expect("path");
        }

        let all_referrers = store.referrers("carlok/alpha").await.expect("referrers");
        assert_eq!(all_referrers.len(), 2, "full history keeps every snapshot");

        let latest_referrers = store
            .latest_referrers("carlok/alpha")
            .await
            .expect("latest referrers");
        assert_eq!(latest_referrers.len(), 1);
        assert_eq!(latest_referrers[0].captured_on, "2026-08-24");

        let latest_paths = store
            .latest_paths("carlok/alpha")
            .await
            .expect("latest paths");
        assert_eq!(latest_paths.len(), 1);
        assert_eq!(latest_paths[0].captured_on, "2026-08-24");
    }

    #[tokio::test]
    async fn charts_scope_clones_and_views_to_the_given_repositories() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let database = format!("sqlite:{}", directory.path().join("traffic.db").display());
        let store = Store::connect(&database).await.expect("connect");
        for (name, clones, views) in [("carlok/a", 4, 9), ("carlok/b", 6, 1)] {
            store
                .upsert_repository(&Repository {
                    name: name.into(),
                    description: String::new(),
                    stars: 0,
                    forks: 0,
                    watchers: 0,
                    issues: 0,
                    pull_requests: 0,
                    is_fork: false,
                    is_archived: false,
                    created_at: "2026-08-01".into(),
                    updated_at: "2026-08-01".into(),
                })
                .await
                .expect("repository");
            store
                .upsert_daily_traffic(name, "2026-08-01", "clone", clones, clones / 2)
                .await
                .expect("clone traffic");
            store
                .upsert_daily_traffic(name, "2026-08-01", "view", views, views / 3)
                .await
                .expect("view traffic");
        }
        let summaries = store.repository_summaries("").await.expect("summaries");

        let clone_chart = store.clone_chart(&summaries).await.expect("clone chart");
        assert_eq!(clone_chart.len(), 1);
        assert_eq!(clone_chart[0].total_clones, 10);
        assert_eq!(clone_chart[0].unique_cloners, 5);

        let views_chart = store.views_chart(&summaries).await.expect("views chart");
        assert_eq!(views_chart.len(), 1);
        assert_eq!(views_chart[0].count, 10);
        assert_eq!(views_chart[0].uniques, 3);
    }

    #[tokio::test]
    async fn charts_are_empty_for_no_matched_repositories() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let database = format!("sqlite:{}", directory.path().join("traffic.db").display());
        let store = Store::connect(&database).await.expect("connect");
        assert!(
            store
                .clone_chart(&[])
                .await
                .expect("clone chart")
                .is_empty()
        );
        assert!(
            store
                .views_chart(&[])
                .await
                .expect("views chart")
                .is_empty()
        );
    }

    #[tokio::test]
    async fn records_completed_sync_runs() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let database = format!("sqlite:{}", directory.path().join("traffic.db").display());
        let store = Store::connect(&database).await.expect("connect");
        let id = store.start_sync_run().await.expect("start run");
        store
            .finish_sync_run(id, "succeeded", 3, None)
            .await
            .expect("finish run");
        let row: (String, i64) =
            sqlx::query_as("SELECT status, repositories_synced FROM sync_runs WHERE id=?")
                .bind(id)
                .fetch_one(store.pool())
                .await
                .expect("stored row");
        assert_eq!(row, ("succeeded".to_string(), 3));
    }

    #[tokio::test]
    async fn assigns_equal_clone_totals_the_same_rank() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let database = format!("sqlite:{}", directory.path().join("traffic.db").display());
        let store = Store::connect(&database).await.expect("connect");
        for (name, clones) in [
            ("carlok/a", 8),
            ("carlok/b", 5),
            ("carlok/c", 5),
            ("carlok/d", 1),
        ] {
            store
                .upsert_repository(&Repository {
                    name: name.into(),
                    description: String::new(),
                    stars: 0,
                    forks: 0,
                    watchers: 0,
                    issues: 0,
                    pull_requests: 0,
                    is_fork: false,
                    is_archived: false,
                    created_at: "2026-08-01".into(),
                    updated_at: "2026-08-01".into(),
                })
                .await
                .expect("repository");
            store
                .upsert_daily_traffic(name, "2026-08-01", "clone", clones, 0)
                .await
                .expect("traffic");
        }
        let ranks = store
            .repository_summaries("")
            .await
            .expect("summaries")
            .into_iter()
            .map(|summary| summary.clone_rank)
            .collect::<Vec<_>>();
        assert_eq!(ranks, vec![1, 2, 2, 4]);
    }

    #[test]
    fn dense_rank_breaks_ties_by_name_and_skips_ranks_after_a_tie() {
        let totals = BTreeMap::from([
            ("b".to_string(), 5),
            ("a".to_string(), 5),
            ("c".to_string(), 1),
        ]);
        let ranks = dense_rank_by_total(totals);
        assert_eq!(ranks["a"], 1);
        assert_eq!(ranks["b"], 1);
        assert_eq!(ranks["c"], 3);
    }

    #[test]
    fn dense_rank_by_total_of_empty_map_is_empty() {
        assert!(dense_rank_by_total(BTreeMap::new()).is_empty());
    }

    #[test]
    fn dense_rank_by_score_omits_repositories_with_no_score() {
        let scores = BTreeMap::from([
            ("scored".to_string(), Some(4.0)),
            ("unscored".to_string(), None),
        ]);
        let ranks = dense_rank_by_score(scores);
        assert_eq!(ranks["scored"], 1);
        assert!(!ranks.contains_key("unscored"));
    }

    #[test]
    fn compare_rank_reports_up_down_and_stable() {
        assert_eq!(compare_rank(1, 2), RankTrend::Up);
        assert_eq!(compare_rank(2, 1), RankTrend::Down);
        assert_eq!(compare_rank(3, 3), RankTrend::Stable);
    }

    #[test]
    fn existed_as_of_treats_an_unparseable_date_as_having_existed() {
        assert!(existed_as_of("", "2026-09-14"));
        assert!(existed_as_of("2026-09-01", "2026-09-14"));
        assert!(!existed_as_of("2026-09-14", "2026-09-13"));
    }

    async fn insert_bare_repository(store: &Store, name: &str, created_at: &str) {
        store
            .upsert_repository(&Repository {
                name: name.into(),
                description: String::new(),
                stars: 0,
                forks: 0,
                watchers: 0,
                issues: 0,
                pull_requests: 0,
                is_fork: false,
                is_archived: false,
                created_at: created_at.into(),
                updated_at: created_at.into(),
            })
            .await
            .expect("repository");
    }

    #[tokio::test]
    async fn clone_rank_trend_reflects_an_overtake_since_yesterday() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let store = Store::connect(&format!(
            "sqlite:{}",
            directory.path().join("traffic.db").display()
        ))
        .await
        .expect("store");
        let today = Utc::now().date_naive();
        let yesterday = today - Duration::days(1);
        let old = (today - Duration::days(60)).to_string();

        // "climber" trails as of yesterday, then gets a burst of clones today and overtakes.
        insert_bare_repository(&store, "carlok/climber", &old).await;
        store
            .upsert_daily_traffic("carlok/climber", &yesterday.to_string(), "clone", 5, 0)
            .await
            .expect("clones");
        store
            .upsert_daily_traffic("carlok/climber", &today.to_string(), "clone", 100, 0)
            .await
            .expect("clones");

        // "leader" is ahead as of yesterday and gets nothing new today, so it falls behind.
        insert_bare_repository(&store, "carlok/leader", &old).await;
        store
            .upsert_daily_traffic("carlok/leader", &yesterday.to_string(), "clone", 10, 0)
            .await
            .expect("clones");

        let summaries = store.repository_summaries("").await.expect("summaries");
        let climber = summaries
            .iter()
            .find(|summary| summary.repository.name == "carlok/climber")
            .expect("climber");
        let leader = summaries
            .iter()
            .find(|summary| summary.repository.name == "carlok/leader")
            .expect("leader");
        assert_eq!(climber.clone_rank_trend, RankTrend::Up);
        assert_eq!(leader.clone_rank_trend, RankTrend::Down);
    }

    #[tokio::test]
    async fn clone_rank_trend_is_stable_when_order_is_unchanged() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let store = Store::connect(&format!(
            "sqlite:{}",
            directory.path().join("traffic.db").display()
        ))
        .await
        .expect("store");
        let today = Utc::now().date_naive();
        let old = (today - Duration::days(60)).to_string();

        insert_bare_repository(&store, "carlok/steady-a", &old).await;
        store
            .upsert_daily_traffic("carlok/steady-a", &old, "clone", 100, 0)
            .await
            .expect("clones");
        insert_bare_repository(&store, "carlok/steady-b", &old).await;
        store
            .upsert_daily_traffic("carlok/steady-b", &old, "clone", 50, 0)
            .await
            .expect("clones");

        let summaries = store.repository_summaries("").await.expect("summaries");
        assert!(
            summaries
                .iter()
                .all(|summary| summary.clone_rank_trend == RankTrend::Stable)
        );
    }

    #[tokio::test]
    async fn clone_rank_trend_is_unknown_for_a_repository_created_today() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let store = Store::connect(&format!(
            "sqlite:{}",
            directory.path().join("traffic.db").display()
        ))
        .await
        .expect("store");
        let today = Utc::now().date_naive().to_string();

        insert_bare_repository(&store, "carlok/brand-new", &today).await;
        store
            .upsert_daily_traffic("carlok/brand-new", &today, "clone", 50, 0)
            .await
            .expect("clones");

        let mut summaries = store.repository_summaries("").await.expect("summaries");
        let summary = summaries.remove(0);
        assert_eq!(summary.clone_rank_trend, RankTrend::Unknown);
    }

    #[tokio::test]
    async fn attention_rank_trend_is_unknown_when_yesterdays_baseline_row_is_missing() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let store = Store::connect(&format!(
            "sqlite:{}",
            directory.path().join("traffic.db").display()
        ))
        .await
        .expect("store");
        let today = Utc::now().date_naive();
        let old = (today - Duration::days(60)).to_string();
        let name = "carlok/fresh-history";

        insert_bare_repository(&store, name, &old).await;
        store
            .upsert_referrer(
                name,
                &(today - Duration::days(1)).to_string(),
                "example.com",
                5,
                3,
            )
            .await
            .expect("referrer");
        // Only a baseline exactly 30 days before *today* exists — 30 days before *yesterday*
        // (one day earlier) has no row, so the "as of yesterday" score can't be computed.
        store
            .upsert_star(name, &(today - Duration::days(30)).to_string(), 1)
            .await
            .expect("star baseline");
        store
            .upsert_fork(name, &(today - Duration::days(30)).to_string(), 1)
            .await
            .expect("fork baseline");

        let mut summaries = store.repository_summaries("").await.expect("summaries");
        let summary = summaries.remove(0);
        assert!(summary.human_attention.score.is_some());
        assert_eq!(summary.attention_rank_trend, RankTrend::Unknown);
    }

    #[tokio::test]
    async fn attention_rank_trend_reflects_a_score_order_flip_since_yesterday() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let store = Store::connect(&format!(
            "sqlite:{}",
            directory.path().join("traffic.db").display()
        ))
        .await
        .expect("store");
        let today = Utc::now().date_naive();
        let yesterday = today - Duration::days(1);
        let old = (today - Duration::days(60)).to_string();

        // "riser": flat referrers, flat (zero-delta) stars, and a fork count that jumps only as
        // of today — its score should overtake "steady" today despite trailing yesterday.
        insert_bare_repository(&store, "carlok/riser", &old).await;
        store
            .upsert_referrer("carlok/riser", &yesterday.to_string(), "example.com", 5, 2)
            .await
            .expect("referrer");
        store
            .upsert_star("carlok/riser", &(today - Duration::days(31)).to_string(), 0)
            .await
            .expect("star baseline (yesterday)");
        store
            .upsert_star("carlok/riser", &(today - Duration::days(30)).to_string(), 0)
            .await
            .expect("star baseline (today)");
        store
            .upsert_fork("carlok/riser", &(today - Duration::days(31)).to_string(), 1)
            .await
            .expect("fork baseline (yesterday)");
        store
            .upsert_fork("carlok/riser", &(today - Duration::days(30)).to_string(), 1)
            .await
            .expect("fork baseline (today)");
        store
            .upsert_fork("carlok/riser", &today.to_string(), 6)
            .await
            .expect("fork burst (today only)");

        // "steady": a constant view count and nothing else, so its score never moves.
        insert_bare_repository(&store, "carlok/steady", &old).await;
        store
            .upsert_daily_traffic("carlok/steady", &yesterday.to_string(), "view", 0, 50)
            .await
            .expect("views");
        store
            .upsert_referrer("carlok/steady", &yesterday.to_string(), "github.com", 5, 5)
            .await
            .expect("referrer (filtered out, just establishes a snapshot day)");
        store
            .upsert_star(
                "carlok/steady",
                &(today - Duration::days(31)).to_string(),
                0,
            )
            .await
            .expect("star baseline (yesterday)");
        store
            .upsert_star(
                "carlok/steady",
                &(today - Duration::days(30)).to_string(),
                0,
            )
            .await
            .expect("star baseline (today)");
        store
            .upsert_fork(
                "carlok/steady",
                &(today - Duration::days(31)).to_string(),
                0,
            )
            .await
            .expect("fork baseline (yesterday)");
        store
            .upsert_fork(
                "carlok/steady",
                &(today - Duration::days(30)).to_string(),
                0,
            )
            .await
            .expect("fork baseline (today)");

        let summaries = store.repository_summaries("").await.expect("summaries");
        let riser = summaries
            .iter()
            .find(|summary| summary.repository.name == "carlok/riser")
            .expect("riser");
        let steady = summaries
            .iter()
            .find(|summary| summary.repository.name == "carlok/steady")
            .expect("steady");
        assert_eq!(riser.attention_rank_trend, RankTrend::Up);
        assert_eq!(steady.attention_rank_trend, RankTrend::Down);
    }
}
