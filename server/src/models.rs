use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct Repository {
    pub name: String,
    pub description: String,
    pub stars: i64,
    pub forks: i64,
    pub watchers: i64,
    pub issues: i64,
    pub pull_requests: i64,
    pub is_fork: bool,
    pub is_archived: bool,
    pub updated_at: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct DayPoint {
    pub day: String,
    pub count: i64,
    pub uniques: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct ReferrerPoint {
    pub captured_on: String,
    pub referrer: String,
    pub count: i64,
    pub uniques: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct PathPoint {
    pub captured_on: String,
    pub path: String,
    pub title: String,
    pub count: i64,
    pub uniques: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct StarPoint {
    pub day: String,
    pub total: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RepositorySummary {
    #[serde(flatten)]
    pub repository: Repository,
    pub total_views: i64,
    pub total_view_uniques: i64,
    pub total_clones: i64,
    pub total_clone_uniques: i64,
    pub clones_1d: i64,
    pub clones_7d: i64,
    pub clones_30d: i64,
    pub clone_rank: usize,
    pub clone_share_percent: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CloneStatistics {
    pub mean: f64,
    pub median: f64,
    pub population_variance: f64,
    pub population_standard_deviation: f64,
    pub minimum: i64,
    pub maximum: i64,
    pub p95: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CloneChartPoint {
    pub day: String,
    pub total_clones: i64,
    pub unique_cloners: i64,
}

#[derive(Clone, Debug, Serialize)]
pub struct DashboardResponse {
    pub items: Vec<RepositorySummary>,
    pub total_count: usize,
    pub total_stars: i64,
    pub total_forks: i64,
    pub total_views: i64,
    pub total_clones: i64,
    pub chart: Vec<CloneChartPoint>,
    pub views_chart: Vec<DayPoint>,
    pub total_clone_statistics: Option<CloneStatistics>,
    pub unique_clone_statistics: Option<CloneStatistics>,
}

#[derive(Clone, Debug, Serialize)]
pub struct RepositoryDetail {
    pub summary: RepositorySummary,
    pub clones: Vec<DayPoint>,
    pub views: Vec<DayPoint>,
    pub referrers: Vec<ReferrerPoint>,
    pub paths: Vec<PathPoint>,
    pub stars: Vec<StarPoint>,
}

#[derive(Clone, Debug, Serialize)]
pub struct JsonlExportRow {
    pub repository: RepositorySummary,
    pub clones: Vec<DayPoint>,
    pub views: Vec<DayPoint>,
    pub referrers: Vec<ReferrerPoint>,
    pub paths: Vec<PathPoint>,
    pub stars: Vec<StarPoint>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct SyncRun {
    pub id: i64,
    pub started_at: String,
    pub finished_at: Option<String>,
    pub status: String,
    pub repositories_synced: i64,
    pub message: Option<String>,
}
