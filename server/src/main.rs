use std::{env, net::SocketAddr, sync::Arc, time::Duration};

use anyhow::Context;
use forgepulse_server::{
    api::{AppState, router},
    github::GitHubCollector,
    store::Store,
};
use tokio::signal;
use tracing::{error, info};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();
    let database_url =
        env::var("FORGEPULSE_DB").unwrap_or_else(|_| "sqlite:/data/forgepulse.db".to_string());
    let store = Store::connect(&database_url).await?;
    if env::var("FORGEPULSE_DEMO").as_deref() == Ok("true") {
        store.seed_demo().await?;
    }
    let collector = env::var("FORGEPULSE_GITHUB_TOKEN")
        .ok()
        .filter(|token| !token.is_empty())
        .map(|token| {
            GitHubCollector::new(
                token,
                env::var("FORGEPULSE_FILTER").unwrap_or_else(|_| "carlok/*".to_string()),
            )
        })
        .transpose()?;
    let state = AppState {
        store: store.clone(),
        collector: collector.map(Arc::new),
    };
    if let Some(collector) = state.collector.clone() {
        let scheduled_store = store.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(sync_interval());
            loop {
                interval.tick().await;
                if let Err(error) = collector.sync(&scheduled_store).await {
                    error!(%error, "scheduled sync failed");
                }
            }
        });
    }
    let host = env::var("FORGEPULSE_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let port = env::var("FORGEPULSE_PORT").unwrap_or_else(|_| "18744".to_string());
    let address: SocketAddr = format!("{host}:{port}")
        .parse()
        .context("parse FORGEPULSE_HOST/FORGEPULSE_PORT")?;
    let listener = tokio::net::TcpListener::bind(address).await?;
    info!(%address, "ForgePulse listening");
    let web_dir = env::var("FORGEPULSE_WEB_DIR")
        .ok()
        .filter(|path| !path.is_empty());
    axum::serve(listener, router(state, web_dir))
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    Ok(())
}

fn sync_interval() -> Duration {
    let raw = env::var("FORGEPULSE_SYNC_INTERVAL").unwrap_or_else(|_| "1h".to_string());
    parse_sync_interval(&raw)
}

fn parse_sync_interval(raw: &str) -> Duration {
    let seconds = raw
        .strip_suffix('h')
        .and_then(|value| value.parse::<u64>().ok())
        .map(|hours| hours * 3600)
        .or_else(|| {
            raw.strip_suffix('m')
                .and_then(|value| value.parse::<u64>().ok())
                .map(|minutes| minutes * 60)
        })
        .unwrap_or(3600);
    Duration::from_secs(seconds.max(60))
}

async fn shutdown_signal() {
    let interrupt = async { signal::ctrl_c().await.expect("install Ctrl-C handler") };
    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("install SIGTERM handler")
            .recv()
            .await;
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();
    tokio::select! { _ = interrupt => {}, _ = terminate => {} }
}

#[cfg(test)]
mod tests {
    use super::parse_sync_interval;

    #[test]
    fn accepts_hour_and_minute_intervals() {
        assert_eq!(parse_sync_interval("2h").as_secs(), 7_200);
        assert_eq!(parse_sync_interval("15m").as_secs(), 900);
        assert_eq!(parse_sync_interval("bad").as_secs(), 3_600);
    }
}
