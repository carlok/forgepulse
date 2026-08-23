use std::{collections::BTreeMap, str::FromStr};

use anyhow::Context;
use chrono::{Duration, NaiveDate, Utc};
use sqlx::{
    SqlitePool,
    sqlite::{SqliteConnectOptions, SqliteJournalMode},
};

use crate::{
    models::{
        CloneChartPoint, DayPoint, PathPoint, ReferrerPoint, Repository, RepositorySummary,
        StarPoint, SyncRun,
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
              (name, description, stars, forks, watchers, issues, pull_requests, is_fork, is_archived, updated_at)
              VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
              ON CONFLICT(name) DO UPDATE SET
                description=excluded.description, stars=excluded.stars, forks=excluded.forks,
                watchers=excluded.watchers, issues=excluded.issues, pull_requests=excluded.pull_requests,
                is_fork=excluded.is_fork, is_archived=excluded.is_archived, updated_at=excluded.updated_at"#,
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
                 r.is_fork, r.is_archived, r.updated_at,
                 COALESCE(SUM(CASE WHEN d.metric='view' THEN d.count END), 0) AS total_views,
                 COALESCE(SUM(CASE WHEN d.metric='view' THEN d.uniques END), 0) AS total_view_uniques,
                 COALESCE(SUM(CASE WHEN d.metric='clone' THEN d.count END), 0) AS total_clones,
                 COALESCE(SUM(CASE WHEN d.metric='clone' THEN d.uniques END), 0) AS total_clone_uniques,
                 COALESCE(SUM(CASE WHEN d.metric='clone' AND d.day >= date('now', '-1 day') THEN d.count END), 0) AS clones_1d,
                 COALESCE(SUM(CASE WHEN d.metric='clone' AND d.day >= date('now', '-6 day') THEN d.count END), 0) AS clones_7d,
                 COALESCE(SUM(CASE WHEN d.metric='clone' AND d.day >= date('now', '-29 day') THEN d.count END), 0) AS clones_30d
               FROM repositories r LEFT JOIN daily_traffic d ON d.repository_name=r.name
               WHERE lower(r.name) LIKE lower(?)
               GROUP BY r.name ORDER BY total_clones DESC, r.name ASC"#,
        )
        .bind(format!("%{}%", search.trim()))
        .fetch_all(&self.pool)
        .await?;
        let total_clones = rows.iter().map(|row| row.total_clones).sum::<i64>();
        let mut summaries = Vec::with_capacity(rows.len());
        let mut previous_total = None;
        let mut rank = 0;
        for (index, row) in rows.into_iter().enumerate() {
            if previous_total != Some(row.total_clones) {
                rank = index + 1;
                previous_total = Some(row.total_clones);
            }
            summaries.push(RepositorySummary {
                repository: Repository {
                    name: row.name,
                    description: row.description,
                    stars: row.stars,
                    forks: row.forks,
                    watchers: row.watchers,
                    issues: row.issues,
                    pull_requests: row.pull_requests,
                    is_fork: row.is_fork,
                    is_archived: row.is_archived,
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
                clone_share_percent: if total_clones == 0 {
                    0.0
                } else {
                    row.total_clones as f64 * 100.0 / total_clones as f64
                },
            });
        }
        Ok(summaries)
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

    pub async fn paths(&self, repository_name: &str) -> anyhow::Result<Vec<PathPoint>> {
        Ok(sqlx::query_as("SELECT captured_on, path, title, count, uniques FROM path_snapshots WHERE repository_name=? ORDER BY captured_on DESC, count DESC")
            .bind(repository_name).fetch_all(&self.pool).await?)
    }

    pub async fn stars(&self, repository_name: &str) -> anyhow::Result<Vec<StarPoint>> {
        Ok(sqlx::query_as(
            "SELECT day, total FROM star_history WHERE repository_name=? ORDER BY day",
        )
        .bind(repository_name)
        .fetch_all(&self.pool)
        .await?)
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
    updated_at: String,
    total_views: i64,
    total_view_uniques: i64,
    total_clones: i64,
    total_clone_uniques: i64,
    clones_1d: i64,
    clones_7d: i64,
    clones_30d: i64,
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
}
