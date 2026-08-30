ALTER TABLE repositories ADD COLUMN created_at TEXT NOT NULL DEFAULT '';

CREATE TABLE fork_history (
  repository_name TEXT NOT NULL REFERENCES repositories(name) ON DELETE CASCADE,
  day TEXT NOT NULL,
  total INTEGER NOT NULL CHECK (total >= 0),
  PRIMARY KEY (repository_name, day)
);

CREATE TABLE release_events (
  repository_name TEXT NOT NULL REFERENCES repositories(name) ON DELETE CASCADE,
  kind TEXT NOT NULL CHECK (kind IN ('release', 'tag')),
  name TEXT NOT NULL,
  occurred_on TEXT NOT NULL,
  PRIMARY KEY (repository_name, kind, name)
);

CREATE TABLE actions_checkout_snapshots (
  repository_name TEXT NOT NULL REFERENCES repositories(name) ON DELETE CASCADE,
  captured_on TEXT NOT NULL,
  completed_checkout_jobs INTEGER NOT NULL CHECK (completed_checkout_jobs >= 0),
  PRIMARY KEY (repository_name, captured_on)
);
