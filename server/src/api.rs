use std::{env, sync::Arc};

use axum::{
    Json, Router,
    body::{Body, Bytes},
    extract::{Path, Query, State},
    http::{HeaderValue, StatusCode, header},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use chrono::Utc;
use futures_util::{StreamExt, stream};
use serde::Deserialize;
use tower_http::{
    cors::CorsLayer,
    services::{ServeDir, ServeFile},
    trace::TraceLayer,
};

use crate::{
    github::GitHubCollector,
    models::{DashboardResponse, JsonlExportRow, RepositoryDetail, RepositorySummary, SyncRun},
    store::{Store, statistics_for_chart},
};

#[derive(Clone)]
pub struct AppState {
    pub store: Store,
    pub collector: Option<Arc<GitHubCollector>>,
}

#[derive(Default, Deserialize)]
pub struct DashboardQuery {
    pub q: Option<String>,
    pub sort: Option<String>,
    pub dir: Option<String>,
    pub page: Option<usize>,
    pub per_page: Option<usize>,
}

pub fn router(state: AppState, web_dir: Option<String>) -> Router {
    let router = Router::new()
        .route("/api/health", get(health))
        .route("/api/v1/health", get(health))
        .route("/api/v1/dashboard", get(dashboard))
        .route(
            "/api/v1/repositories/{owner}/{repository}",
            get(repository_detail),
        )
        .route("/api/v1/export.jsonl", get(export_jsonl))
        .route("/api/v1/sync", post(sync_now))
        .route("/api/v1/sync-runs", get(sync_runs))
        .with_state(state)
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http());
    match web_dir {
        Some(path) => {
            // Any path that isn't a real static asset (e.g. /repositories/owner/repo, reached
            // by right-click "open in new tab" or a bookmarked/typed URL rather than SPA
            // client-side routing) falls back to index.html so the client router can take over.
            let index = ServeFile::new(format!("{path}/index.html"));
            let serve_dir = ServeDir::new(path)
                .append_index_html_on_directories(true)
                .fallback(index);
            router.fallback_service(serve_dir)
        }
        None => router,
    }
}

async fn health() -> Json<serde_json::Value> {
    let git_ref = env::var("FORGEPULSE_GIT_REF").unwrap_or_else(|_| "local".to_string());
    Json(serde_json::json!({"status": "ok", "git_ref": git_ref}))
}

async fn dashboard(
    State(state): State<AppState>,
    Query(query): Query<DashboardQuery>,
) -> Result<Json<DashboardResponse>, ApiError> {
    let search = query.q.unwrap_or_default();
    let mut all = state.store.repository_summaries(&search).await?;
    let chart = state.store.clone_chart(&all).await?;
    let views_chart = state.store.views_chart(&all).await?;
    let total_stars = all.iter().map(|item| item.repository.stars).sum();
    let total_forks = all.iter().map(|item| item.repository.forks).sum();
    let total_views = all.iter().map(|item| item.total_views).sum();
    let total_clones = all.iter().map(|item| item.total_clones).sum();
    sort_repositories(&mut all, query.sort.as_deref(), query.dir.as_deref());
    let total_count = all.len();
    let page = query.page.unwrap_or(1).max(1);
    let per_page = query.per_page.unwrap_or(25).clamp(1, 100);
    let start = ((page - 1) * per_page).min(total_count);
    let items = all.into_iter().skip(start).take(per_page).collect();
    let (total_clone_statistics, unique_clone_statistics) = statistics_for_chart(&chart);
    Ok(Json(DashboardResponse {
        items,
        total_count,
        total_stars,
        total_forks,
        total_views,
        total_clones,
        chart,
        views_chart,
        total_clone_statistics,
        unique_clone_statistics,
    }))
}

async fn repository_detail(
    State(state): State<AppState>,
    Path((owner, repository)): Path<(String, String)>,
) -> Result<Json<RepositoryDetail>, ApiError> {
    let name = format!("{owner}/{repository}");
    let summary = state
        .store
        .repository_summaries(&name)
        .await?
        .into_iter()
        .find(|summary| summary.repository.name == name)
        .ok_or(ApiError::not_found())?;
    Ok(Json(RepositoryDetail {
        summary,
        clones: state.store.traffic(&name, "clone").await?,
        views: state.store.traffic(&name, "view").await?,
        referrers: state.store.latest_referrers(&name).await?,
        paths: state.store.latest_paths(&name).await?,
        stars: state.store.stars(&name).await?,
    }))
}

async fn export_jsonl(
    State(state): State<AppState>,
    Query(query): Query<DashboardQuery>,
) -> Result<Response, ApiError> {
    let repositories = state
        .store
        .repository_summaries(query.q.as_deref().unwrap_or_default())
        .await?;
    let store = state.store.clone();
    let lines = stream::iter(repositories).then(move |repository| {
        let store = store.clone();
        async move {
            let name = repository.repository.name.clone();
            let row = JsonlExportRow {
                repository,
                clones: store
                    .traffic(&name, "clone")
                    .await
                    .map_err(to_stream_error)?,
                views: store
                    .traffic(&name, "view")
                    .await
                    .map_err(to_stream_error)?,
                referrers: store.referrers(&name).await.map_err(to_stream_error)?,
                paths: store.paths(&name).await.map_err(to_stream_error)?,
                stars: store.stars(&name).await.map_err(to_stream_error)?,
            };
            let line = serde_json::to_string(&row).map_err(to_stream_error)?;
            Ok::<Bytes, std::io::Error>(Bytes::from(format!("{line}\n")))
        }
    });
    let filename = format!(
        "forgepulse-export-{}.jsonl",
        Utc::now().format("%Y%m%d-%H%M")
    );
    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/x-ndjson; charset=utf-8")
        .header(
            header::CONTENT_DISPOSITION,
            format!("attachment; filename={filename}"),
        )
        .header(header::CACHE_CONTROL, "no-store")
        .body(Body::from_stream(lines))
        .map_err(|error| ApiError::from(anyhow::Error::from(error)))?)
}

fn to_stream_error(error: impl std::fmt::Display) -> std::io::Error {
    std::io::Error::other(error.to_string())
}

async fn sync_now(State(state): State<AppState>) -> Result<Json<serde_json::Value>, ApiError> {
    let collector = state
        .collector
        .ok_or_else(|| ApiError::bad_request("GitHub sync is not configured"))?;
    let count = collector.sync(&state.store).await?;
    Ok(Json(
        serde_json::json!({"status": "ok", "repositories_synced": count}),
    ))
}

async fn sync_runs(State(state): State<AppState>) -> Result<Json<Vec<SyncRun>>, ApiError> {
    Ok(Json(state.store.sync_runs().await?))
}

fn sort_repositories(items: &mut [RepositorySummary], sort: Option<&str>, dir: Option<&str>) {
    let descending = dir != Some("asc");
    let field = sort.unwrap_or("total_clones");
    items.sort_by(|left, right| {
        let order = match field {
            "name" => left.repository.name.cmp(&right.repository.name),
            "stars" => left.repository.stars.cmp(&right.repository.stars),
            "forks" => left.repository.forks.cmp(&right.repository.forks),
            "total_views" => left.total_views.cmp(&right.total_views),
            "clones_1d" => left.clones_1d.cmp(&right.clones_1d),
            "clones_7d" => left.clones_7d.cmp(&right.clones_7d),
            "clones_30d" => left.clones_30d.cmp(&right.clones_30d),
            _ => left.total_clones.cmp(&right.total_clones),
        }
        .then_with(|| left.repository.name.cmp(&right.repository.name));
        if descending { order.reverse() } else { order }
    });
}

#[derive(Debug)]
struct ApiError {
    status: StatusCode,
    message: String,
}

impl ApiError {
    fn not_found() -> Self {
        Self {
            status: StatusCode::NOT_FOUND,
            message: "Not found".to_string(),
        }
    }
    fn bad_request(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            message: message.into(),
        }
    }
}

impl From<anyhow::Error> for ApiError {
    fn from(error: anyhow::Error) -> Self {
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            message: error.to_string(),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let mut response = Json(serde_json::json!({"error": self.message})).into_response();
        *response.status_mut() = self.status;
        response.headers_mut().insert(
            header::X_CONTENT_TYPE_OPTIONS,
            HeaderValue::from_static("nosniff"),
        );
        response
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{body::to_bytes, http::Request};
    use tower::ServiceExt;

    #[test]
    fn sorts_descending_by_default() {
        let mut items = vec![summary("b", 1), summary("a", 2)];
        sort_repositories(&mut items, None, None);
        assert_eq!(items[0].repository.name, "a");
    }

    fn summary(name: &str, clones: i64) -> RepositorySummary {
        RepositorySummary {
            repository: crate::models::Repository {
                name: name.to_string(),
                description: String::new(),
                stars: 0,
                forks: 0,
                watchers: 0,
                issues: 0,
                pull_requests: 0,
                is_fork: false,
                is_archived: false,
                updated_at: String::new(),
            },
            total_views: 0,
            total_view_uniques: 0,
            total_clones: clones,
            total_clone_uniques: 0,
            clones_1d: 0,
            clones_7d: 0,
            clones_30d: 0,
            clone_rank: 1,
            clone_share_percent: 0.0,
        }
    }

    #[tokio::test]
    async fn dashboard_embeds_scoped_chart_and_statistics() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let database = format!("sqlite:{}", directory.path().join("traffic.db").display());
        let store = Store::connect(&database).await.expect("store");
        for (name, clones) in [("carlok/alpha", 5), ("carlok/beta", 9)] {
            store
                .upsert_repository(&crate::models::Repository {
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
                .upsert_daily_traffic(name, "2026-08-01", "clone", clones, clones / 2)
                .await
                .expect("traffic");
            store
                .upsert_daily_traffic(name, "2026-08-01", "view", clones * 3, clones)
                .await
                .expect("traffic");
        }
        let response = router(
            AppState {
                store,
                collector: None,
            },
            None,
        )
        .oneshot(
            Request::builder()
                .uri("/api/v1/dashboard?q=alpha")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
        assert_eq!(response.status(), StatusCode::OK);
        let json: serde_json::Value = serde_json::from_slice(
            &to_bytes(response.into_body(), usize::MAX)
                .await
                .expect("body"),
        )
        .expect("json");
        assert_eq!(json["total_count"], 1);
        assert_eq!(json["items"][0]["clone_rank"], 1);
        assert_eq!(json["chart"][0]["total_clones"], 5);
        assert_eq!(json["chart"][0]["unique_cloners"], 2);
        assert_eq!(json["views_chart"][0]["count"], 15);
        assert_eq!(json["views_chart"][0]["uniques"], 5);
        assert_eq!(json["total_clone_statistics"]["mean"], 5.0);
    }

    #[tokio::test]
    async fn jsonl_export_streams_only_the_requested_repository() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let database = format!("sqlite:{}", directory.path().join("traffic.db").display());
        let store = Store::connect(&database).await.expect("store");
        for name in ["carlok/alpha", "carlok/beta"] {
            store
                .upsert_repository(&crate::models::Repository {
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
        }
        let response = router(
            AppState {
                store,
                collector: None,
            },
            None,
        )
        .oneshot(
            Request::builder()
                .uri("/api/v1/export.jsonl?q=alpha")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
        assert_eq!(response.status(), StatusCode::OK);
        let disposition = response
            .headers()
            .get(header::CONTENT_DISPOSITION)
            .expect("content-disposition header")
            .to_str()
            .expect("ascii header")
            .to_string();
        assert!(
            disposition.starts_with("attachment; filename=forgepulse-export-")
                && disposition.ends_with(".jsonl"),
            "unexpected content-disposition: {disposition}"
        );
        let body = String::from_utf8(
            to_bytes(response.into_body(), usize::MAX)
                .await
                .expect("body")
                .to_vec(),
        )
        .expect("utf-8");
        let lines = body.lines().collect::<Vec<_>>();
        assert_eq!(lines.len(), 1);
        let record: serde_json::Value = serde_json::from_str(lines[0]).expect("jsonl record");
        assert_eq!(record["repository"]["name"], "carlok/alpha");
    }

    #[tokio::test]
    async fn dashboard_returns_empty_charts_when_nothing_matches() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let database = format!("sqlite:{}", directory.path().join("traffic.db").display());
        let store = Store::connect(&database).await.expect("store");
        store
            .upsert_repository(&crate::models::Repository {
                name: "carlok/alpha".into(),
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
        let response = router(
            AppState {
                store,
                collector: None,
            },
            None,
        )
        .oneshot(
            Request::builder()
                .uri("/api/v1/dashboard?q=no-such-repository")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
        assert_eq!(response.status(), StatusCode::OK);
        let json: serde_json::Value = serde_json::from_slice(
            &to_bytes(response.into_body(), usize::MAX)
                .await
                .expect("body"),
        )
        .expect("json");
        assert_eq!(json["total_count"], 0);
        assert_eq!(json["chart"], serde_json::json!([]));
        assert_eq!(json["views_chart"], serde_json::json!([]));
        assert_eq!(json["total_clone_statistics"], serde_json::Value::Null);
    }

    #[tokio::test]
    async fn repository_detail_returns_not_found_for_unknown_repository() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let database = format!("sqlite:{}", directory.path().join("traffic.db").display());
        let store = Store::connect(&database).await.expect("store");
        let response = router(
            AppState {
                store,
                collector: None,
            },
            None,
        )
        .oneshot(
            Request::builder()
                .uri("/api/v1/repositories/carlok/missing")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        let json: serde_json::Value = serde_json::from_slice(
            &to_bytes(response.into_body(), usize::MAX)
                .await
                .expect("body"),
        )
        .expect("json");
        assert_eq!(json["error"], "Not found");
    }

    #[tokio::test]
    async fn health_reports_a_git_ref() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let database = format!("sqlite:{}", directory.path().join("traffic.db").display());
        let store = Store::connect(&database).await.expect("store");
        let response = router(
            AppState {
                store,
                collector: None,
            },
            None,
        )
        .oneshot(
            Request::builder()
                .uri("/api/health")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
        assert_eq!(response.status(), StatusCode::OK);
        let json: serde_json::Value = serde_json::from_slice(
            &to_bytes(response.into_body(), usize::MAX)
                .await
                .expect("body"),
        )
        .expect("json");
        assert_eq!(json["status"], "ok");
        // Defaults to "local" when FORGEPULSE_GIT_REF isn't set, which is the case in this
        // process — mutating env vars isn't safe to do per-test in a parallel test binary.
        assert_eq!(json["git_ref"], "local");
    }

    #[tokio::test]
    async fn unmatched_client_routes_fall_back_to_index_html() {
        let db_directory = tempfile::tempdir().expect("temporary directory");
        let database = format!(
            "sqlite:{}",
            db_directory.path().join("traffic.db").display()
        );
        let store = Store::connect(&database).await.expect("store");

        let web_directory = tempfile::tempdir().expect("temporary web directory");
        std::fs::write(
            web_directory.path().join("index.html"),
            "<!doctype html><title>ForgePulse</title>",
        )
        .expect("write index.html");

        let response = router(
            AppState {
                store,
                collector: None,
            },
            Some(web_directory.path().display().to_string()),
        )
        .oneshot(
            Request::builder()
                .uri("/repositories/carlok/forgepulse")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
        assert_eq!(response.status(), StatusCode::OK);
        let body = String::from_utf8(
            to_bytes(response.into_body(), usize::MAX)
                .await
                .expect("body")
                .to_vec(),
        )
        .expect("utf-8");
        assert!(body.contains("ForgePulse"));
    }
}
