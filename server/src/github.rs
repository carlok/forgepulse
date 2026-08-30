use std::time::Duration;

use anyhow::Context;
use chrono::{Duration as ChronoDuration, Utc};
use futures_util::{StreamExt, stream};
use reqwest::{Client, StatusCode};
use serde::Deserialize;

use crate::{models::Repository, store::Store};

const GITHUB_API: &str = "https://api.github.com";

#[derive(Clone)]
pub struct GitHubCollector {
    client: Client,
    token: String,
    filter: String,
    base_url: String,
}

#[derive(Debug, Deserialize)]
struct GitHubRepository {
    full_name: String,
    description: Option<String>,
    stargazers_count: i64,
    forks_count: i64,
    #[serde(default)]
    subscribers_count: i64,
    #[serde(default)]
    open_issues_count: i64,
    fork: bool,
    archived: bool,
    #[serde(default)]
    created_at: String,
}

#[derive(Debug, Deserialize)]
struct TrafficResponse {
    #[serde(default)]
    views: Vec<TrafficPoint>,
    #[serde(default)]
    clones: Vec<TrafficPoint>,
}

#[derive(Debug, Deserialize)]
struct TrafficPoint {
    timestamp: String,
    count: i64,
    uniques: i64,
}

#[derive(Debug, Deserialize)]
struct Referrer {
    referrer: String,
    count: i64,
    uniques: i64,
}

#[derive(Debug, Deserialize)]
struct PopularPath {
    path: String,
    title: String,
    count: i64,
    uniques: i64,
}

#[derive(Debug, Deserialize)]
struct GitHubRelease {
    tag_name: String,
    published_at: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GitHubTag {
    name: String,
    commit: GitObject,
}

#[derive(Debug, Deserialize)]
struct GitObject {
    sha: String,
    #[serde(rename = "type")]
    kind: String,
}

#[derive(Debug, Deserialize)]
struct AnnotatedTag {
    tagger: Tagger,
}

#[derive(Debug, Deserialize)]
struct Tagger {
    date: String,
}

#[derive(Debug, Deserialize)]
struct WorkflowRuns {
    #[serde(default)]
    workflow_runs: Vec<WorkflowRun>,
}

#[derive(Debug, Deserialize)]
struct WorkflowRun {
    id: i64,
    #[serde(default)]
    created_at: String,
}

#[derive(Debug, Deserialize)]
struct WorkflowJobs {
    #[serde(default)]
    jobs: Vec<WorkflowJob>,
}

#[derive(Debug, Deserialize)]
struct WorkflowJob {
    #[serde(default)]
    status: String,
    #[serde(default)]
    steps: Vec<WorkflowStep>,
}

#[derive(Debug, Deserialize)]
struct WorkflowStep {
    #[serde(default)]
    name: String,
}

impl GitHubCollector {
    pub fn new(token: String, filter: String) -> anyhow::Result<Self> {
        Self::with_base_url(token, filter, GITHUB_API)
    }

    fn with_base_url(token: String, filter: String, base_url: &str) -> anyhow::Result<Self> {
        let client = Client::builder()
            .user_agent("forgepulse/0.1")
            .timeout(Duration::from_secs(30))
            .build()?;
        Ok(Self {
            client,
            token,
            filter,
            base_url: base_url.trim_end_matches('/').to_string(),
        })
    }

    pub async fn sync(&self, store: &Store) -> anyhow::Result<usize> {
        let run_id = store.start_sync_run().await?;
        let outcome = self.sync_inner(store).await;
        match &outcome {
            Ok(count) => {
                store
                    .finish_sync_run(run_id, "succeeded", *count as i64, None)
                    .await?
            }
            Err(error) => {
                store
                    .finish_sync_run(run_id, "failed", 0, Some(&error.to_string()))
                    .await?
            }
        }
        outcome
    }

    async fn sync_inner(&self, store: &Store) -> anyhow::Result<usize> {
        let repositories: Vec<GitHubRepository> = self
            .get("/user/repos?visibility=public&per_page=100")
            .await?;
        let repositories = repositories
            .into_iter()
            .filter(|repository| self.includes(repository))
            .collect::<Vec<_>>();
        let mut collection = stream::iter(
            repositories
                .into_iter()
                .map(|repository| async move { self.sync_repository(store, &repository).await }),
        )
        .buffer_unordered(4);
        let mut synced = 0;
        while let Some(result) = collection.next().await {
            result?;
            synced += 1;
        }
        Ok(synced)
    }

    async fn sync_repository(
        &self,
        store: &Store,
        source: &GitHubRepository,
    ) -> anyhow::Result<()> {
        let today = Utc::now().date_naive().to_string();
        let pull_count: i64 = self
            .get::<Vec<serde_json::Value>>(&format!(
                "/repos/{}/pulls?state=open&per_page=100",
                source.full_name
            ))
            .await
            .map(|pulls| pulls.len() as i64)
            .unwrap_or(0);
        store
            .upsert_repository(&Repository {
                name: source.full_name.clone(),
                description: source.description.clone().unwrap_or_default(),
                stars: source.stargazers_count,
                forks: source.forks_count,
                watchers: source.subscribers_count,
                issues: (source.open_issues_count - pull_count).max(0),
                pull_requests: pull_count,
                is_fork: source.fork,
                is_archived: source.archived,
                created_at: day_from_timestamp(&source.created_at).unwrap_or_default(),
                updated_at: today.clone(),
            })
            .await?;

        let views: TrafficResponse = self
            .get(&format!("/repos/{}/traffic/views", source.full_name))
            .await?;
        for point in views.views {
            store
                .upsert_daily_traffic(
                    &source.full_name,
                    &day_from_timestamp(&point.timestamp)?,
                    "view",
                    point.count,
                    point.uniques,
                )
                .await?;
        }
        let clones: TrafficResponse = self
            .get(&format!("/repos/{}/traffic/clones", source.full_name))
            .await?;
        for point in clones.clones {
            store
                .upsert_daily_traffic(
                    &source.full_name,
                    &day_from_timestamp(&point.timestamp)?,
                    "clone",
                    point.count,
                    point.uniques,
                )
                .await?;
        }
        for point in self
            .get::<Vec<Referrer>>(&format!(
                "/repos/{}/traffic/popular/referrers",
                source.full_name
            ))
            .await
            .unwrap_or_default()
        {
            store
                .upsert_referrer(
                    &source.full_name,
                    &today,
                    &point.referrer,
                    point.count,
                    point.uniques,
                )
                .await?;
        }
        for point in self
            .get::<Vec<PopularPath>>(&format!(
                "/repos/{}/traffic/popular/paths",
                source.full_name
            ))
            .await
            .unwrap_or_default()
        {
            store
                .upsert_path(
                    &source.full_name,
                    &today,
                    &point.path,
                    &point.title,
                    point.count,
                    point.uniques,
                )
                .await?;
        }
        store
            .upsert_star(&source.full_name, &today, source.stargazers_count)
            .await?;
        store
            .upsert_fork(&source.full_name, &today, source.forks_count)
            .await?;
        if let Ok(releases) = self
            .get::<Vec<GitHubRelease>>(&format!(
                "/repos/{}/releases?per_page=100",
                source.full_name
            ))
            .await
        {
            for release in releases {
                if let Some(published_at) = release.published_at {
                    store
                        .upsert_release_event(
                            &source.full_name,
                            "release",
                            &release.tag_name,
                            &day_from_timestamp(&published_at)?,
                        )
                        .await?;
                }
            }
        }
        if let Ok(tags) = self
            .get::<Vec<GitHubTag>>(&format!("/repos/{}/tags?per_page=100", source.full_name))
            .await
        {
            for tag in tags.into_iter().filter(|tag| tag.commit.kind == "tag") {
                if let Ok(annotated) = self
                    .get::<AnnotatedTag>(&format!(
                        "/repos/{}/git/tags/{}",
                        source.full_name, tag.commit.sha
                    ))
                    .await
                {
                    store
                        .upsert_release_event(
                            &source.full_name,
                            "tag",
                            &tag.name,
                            &day_from_timestamp(&annotated.tagger.date)?,
                        )
                        .await?;
                }
            }
        }
        if let Ok(estimate) = self.checkout_job_estimate(&source.full_name).await {
            store
                .upsert_actions_checkout_estimate(&source.full_name, &today, estimate)
                .await?;
        }
        Ok(())
    }

    async fn checkout_job_estimate(&self, repository: &str) -> anyhow::Result<i64> {
        let cutoff = Utc::now() - ChronoDuration::days(30);
        let runs: WorkflowRuns = self
            .get(&format!(
                "/repos/{repository}/actions/runs?status=completed&per_page=100"
            ))
            .await?;
        let mut estimate = 0;
        for run in runs.workflow_runs {
            let created = chrono::DateTime::parse_from_rfc3339(&run.created_at)
                .ok()
                .map(|date| date.with_timezone(&Utc));
            if created.is_some_and(|date| date < cutoff) {
                continue;
            }
            let jobs: WorkflowJobs = self
                .get(&format!(
                    "/repos/{repository}/actions/runs/{}/jobs?per_page=100",
                    run.id
                ))
                .await?;
            let uses_checkout = jobs.jobs.iter().any(|job| {
                job.steps
                    .iter()
                    .any(|step| step.name.to_ascii_lowercase().contains("actions/checkout"))
            });
            if uses_checkout {
                estimate += jobs
                    .jobs
                    .iter()
                    .filter(|job| job.status == "completed")
                    .count() as i64;
            }
        }
        Ok(estimate)
    }

    async fn get<T: serde::de::DeserializeOwned>(&self, path: &str) -> anyhow::Result<T> {
        let url = format!("{}{path}", self.base_url);
        for attempt in 0..3 {
            let response = self
                .client
                .get(&url)
                .bearer_auth(&self.token)
                .header("Accept", "application/vnd.github+json")
                .header("X-GitHub-Api-Version", "2022-11-28")
                .send()
                .await?;
            if response.status() == StatusCode::TOO_MANY_REQUESTS
                || response.status() == StatusCode::FORBIDDEN
            {
                if attempt < 2 {
                    tokio::time::sleep(Duration::from_secs(1 << attempt)).await;
                    continue;
                }
            }
            return response
                .error_for_status()?
                .json()
                .await
                .context("decode GitHub response");
        }
        unreachable!("retry loop returns on its final attempt")
    }

    fn includes(&self, repository: &GitHubRepository) -> bool {
        let rules = self
            .filter
            .split(',')
            .map(str::trim)
            .filter(|rule| !rule.is_empty())
            .collect::<Vec<_>>();
        if rules.is_empty() || rules.contains(&"*") {
            return true;
        }
        rules.iter().any(|rule| match rule.strip_suffix("/*") {
            Some(owner) => repository.full_name.starts_with(&format!("{owner}/")),
            None => *rule == repository.full_name,
        })
    }
}

fn day_from_timestamp(timestamp: &str) -> anyhow::Result<String> {
    Ok(chrono::DateTime::parse_from_rfc3339(timestamp)?
        .date_naive()
        .to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        Json, Router,
        http::{StatusCode as HttpStatusCode, Uri},
        response::{IntoResponse, Response},
        routing::get,
    };

    async fn mock_github(uri: Uri) -> Response {
        let body = match uri.path() {
            "/user/repos" => serde_json::json!([{
                "full_name": "carlok/alpha",
                "description": "test repository",
                "stargazers_count": 7,
                "forks_count": 2,
                "open_issues_count": 3,
                "fork": false,
                "archived": false
            }]),
            "/repos/carlok/alpha/pulls" => serde_json::json!([{ "number": 1 }]),
            "/repos/carlok/alpha/traffic/views" => serde_json::json!({
                "views": [{ "timestamp": "2026-08-20T00:00:00Z", "count": 9, "uniques": 4 }]
            }),
            "/repos/carlok/alpha/traffic/clones" => serde_json::json!({
                "clones": [{ "timestamp": "2026-08-20T00:00:00Z", "count": 5, "uniques": 3 }]
            }),
            "/repos/carlok/alpha/traffic/popular/referrers" => serde_json::json!([
                { "referrer": "example.com", "count": 4, "uniques": 2 }
            ]),
            "/repos/carlok/alpha/traffic/popular/paths" => serde_json::json!([
                { "path": "/README.md", "title": "Read me", "count": 6, "uniques": 4 }
            ]),
            _ => return HttpStatusCode::NOT_FOUND.into_response(),
        };
        Json(body).into_response()
    }

    async fn mock_server_url() -> String {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind mock GitHub server");
        let address = listener.local_addr().expect("mock address");
        tokio::spawn(async move {
            axum::serve(listener, Router::new().fallback(get(mock_github)))
                .await
                .expect("serve mock GitHub server");
        });
        format!("http://{address}")
    }

    fn repository(name: &str) -> GitHubRepository {
        GitHubRepository {
            full_name: name.to_string(),
            description: None,
            stargazers_count: 0,
            forks_count: 0,
            subscribers_count: 0,
            open_issues_count: 0,
            fork: false,
            archived: false,
            created_at: String::new(),
        }
    }

    #[test]
    fn filter_accepts_owner_and_exact_repository_rules() {
        let collector =
            GitHubCollector::new("token".into(), "carlok/*,other/one".into()).expect("client");
        assert!(collector.includes(&repository("carlok/alpha")));
        assert!(collector.includes(&repository("other/one")));
        assert!(!collector.includes(&repository("other/two")));
    }

    #[test]
    fn normalizes_github_daily_timestamp() {
        assert_eq!(
            day_from_timestamp("2026-08-20T00:00:00Z").expect("date"),
            "2026-08-20"
        );
    }

    #[tokio::test]
    async fn sync_collects_all_stored_traffic_snapshots() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let database = format!("sqlite:{}", directory.path().join("traffic.db").display());
        let store = Store::connect(&database).await.expect("store");
        let collector = GitHubCollector::with_base_url(
            "token".into(),
            "carlok/*".into(),
            &mock_server_url().await,
        )
        .expect("collector");

        assert_eq!(collector.sync(&store).await.expect("sync"), 1);
        let summary = store
            .repository_summaries("alpha")
            .await
            .expect("summary")
            .pop()
            .expect("repository");
        assert_eq!(summary.total_clones, 5);
        assert_eq!(summary.total_views, 9);
        assert_eq!(summary.repository.issues, 2);
        assert_eq!(
            store
                .referrers("carlok/alpha")
                .await
                .expect("referrers")
                .len(),
            1
        );
        assert_eq!(store.paths("carlok/alpha").await.expect("paths").len(), 1);
        assert_eq!(store.stars("carlok/alpha").await.expect("stars").len(), 1);
        assert_eq!(
            store.sync_runs().await.expect("runs")[0].status,
            "succeeded"
        );
    }

    #[tokio::test]
    async fn sync_records_a_failed_run_when_the_api_is_unreachable() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let database = format!("sqlite:{}", directory.path().join("traffic.db").display());
        let store = Store::connect(&database).await.expect("store");
        let collector =
            GitHubCollector::with_base_url("token".into(), "carlok/*".into(), "http://127.0.0.1:9")
                .expect("collector");
        assert!(collector.sync(&store).await.is_err());
        let run = store.sync_runs().await.expect("runs").pop().expect("run");
        assert_eq!(run.status, "failed");
        assert!(run.message.is_some());
    }
}
