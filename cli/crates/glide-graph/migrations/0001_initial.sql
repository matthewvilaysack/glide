CREATE TABLE people (
  id INTEGER PRIMARY KEY,
  handle TEXT UNIQUE NOT NULL,
  display_name TEXT,
  email TEXT,
  team_id INTEGER REFERENCES teams(id),
  first_seen TEXT,
  last_seen TEXT
);

CREATE TABLE teams (
  id INTEGER PRIMARY KEY,
  slug TEXT UNIQUE NOT NULL,
  display_name TEXT,
  source TEXT NOT NULL
);

CREATE TABLE paths (
  id INTEGER PRIMARY KEY,
  pattern TEXT UNIQUE NOT NULL,
  kind TEXT NOT NULL
);

CREATE TABLE ownership (
  path_id INTEGER NOT NULL REFERENCES paths(id),
  owner_id INTEGER NOT NULL,
  owner_kind TEXT NOT NULL,
  source TEXT NOT NULL,
  weight REAL NOT NULL,
  evidence TEXT,
  observed_at TEXT NOT NULL,
  PRIMARY KEY (path_id, owner_id, owner_kind, source)
);

CREATE TABLE friction_events (
  id INTEGER PRIMARY KEY,
  occurred_at TEXT NOT NULL,
  category TEXT NOT NULL,
  severity INTEGER CHECK (severity BETWEEN 1 AND 5),
  person_handle TEXT,
  subject TEXT NOT NULL,
  notes TEXT
);

CREATE TABLE index_runs (
  id INTEGER PRIMARY KEY,
  started_at TEXT NOT NULL,
  finished_at TEXT,
  files_scanned INTEGER,
  commits_scanned INTEGER,
  ok INTEGER NOT NULL
);

CREATE INDEX idx_ownership_path ON ownership(path_id);
CREATE INDEX idx_friction_occurred ON friction_events(occurred_at);
