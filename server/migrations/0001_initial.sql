PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS repositories (
  name TEXT PRIMARY KEY NOT NULL,
  description TEXT NOT NULL DEFAULT '',
  stars INTEGER NOT NULL DEFAULT 0,
  forks INTEGER NOT NULL DEFAULT 0,
  watchers INTEGER NOT NULL DEFAULT 0,
  issues INTEGER NOT NULL DEFAULT 0,
  pull_requests INTEGER NOT NULL DEFAULT 0,
  is_fork INTEGER NOT NULL DEFAULT 0,
  is_archived INTEGER NOT NULL DEFAULT 0,
  updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS daily_traffic (
  repository_name TEXT NOT NULL REFERENCES repositories(name) ON DELETE CASCADE,
  day TEXT NOT NULL,
  metric TEXT NOT NULL CHECK (metric IN ('clone', 'view')),
  count INTEGER NOT NULL CHECK (count >= 0),
  uniques INTEGER NOT NULL CHECK (uniques >= 0),
  PRIMARY KEY (repository_name, day, metric)
);
CREATE INDEX IF NOT EXISTS daily_traffic_metric_day_idx ON daily_traffic(metric, day);

CREATE TABLE IF NOT EXISTS referrer_snapshots (
  repository_name TEXT NOT NULL REFERENCES repositories(name) ON DELETE CASCADE,
  captured_on TEXT NOT NULL,
  referrer TEXT NOT NULL,
  count INTEGER NOT NULL CHECK (count >= 0),
  uniques INTEGER NOT NULL CHECK (uniques >= 0),
  PRIMARY KEY (repository_name, captured_on, referrer)
);

CREATE TABLE IF NOT EXISTS path_snapshots (
  repository_name TEXT NOT NULL REFERENCES repositories(name) ON DELETE CASCADE,
  captured_on TEXT NOT NULL,
  path TEXT NOT NULL,
  title TEXT NOT NULL DEFAULT '',
  count INTEGER NOT NULL CHECK (count >= 0),
  uniques INTEGER NOT NULL CHECK (uniques >= 0),
  PRIMARY KEY (repository_name, captured_on, path)
);

CREATE TABLE IF NOT EXISTS star_history (
  repository_name TEXT NOT NULL REFERENCES repositories(name) ON DELETE CASCADE,
  day TEXT NOT NULL,
  total INTEGER NOT NULL CHECK (total >= 0),
  PRIMARY KEY (repository_name, day)
);

CREATE TABLE IF NOT EXISTS sync_runs (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  started_at TEXT NOT NULL,
  finished_at TEXT,
  status TEXT NOT NULL CHECK (status IN ('running', 'succeeded', 'failed')),
  repositories_synced INTEGER NOT NULL DEFAULT 0,
  message TEXT
);
