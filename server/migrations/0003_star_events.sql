CREATE TABLE star_events (
  repository_name TEXT NOT NULL REFERENCES repositories(name) ON DELETE CASCADE,
  login TEXT NOT NULL,
  avatar_url TEXT NOT NULL,
  html_url TEXT NOT NULL,
  starred_at TEXT NOT NULL,
  PRIMARY KEY (repository_name, login)
);
CREATE INDEX star_events_starred_at_idx ON star_events(starred_at);
