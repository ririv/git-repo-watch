//! Schema changes are transactional and identified independently of the executable name.
pub(super) const VERSION: i64 = 5;
pub(super) const APPLICATION_ID: i64 = 0x52575053;
pub(super) const CREATE: &str = r#"
CREATE TABLE tasks (
    id TEXT PRIMARY KEY,
    priority INTEGER NOT NULL CHECK(priority IN (0,1,2)),
    started_at TEXT NOT NULL,
    record TEXT NOT NULL CHECK(json_valid(record))
) STRICT;
CREATE INDEX task_priority ON tasks(priority,started_at DESC);
CREATE TABLE batch_reports (
    id INTEGER PRIMARY KEY,
    finished_at TEXT NOT NULL,
    report TEXT NOT NULL CHECK(json_valid(report))
) STRICT;
CREATE TABLE repository_observations (
    path BLOB NOT NULL,
    revision INTEGER NOT NULL CHECK(revision > 0),
    finished_at TEXT NOT NULL,
    report TEXT NOT NULL CHECK(json_valid(report)),
    PRIMARY KEY(path, revision)
) STRICT;
CREATE TABLE repository_heads (
    path BLOB PRIMARY KEY,
    revision INTEGER NOT NULL,
    FOREIGN KEY(path, revision) REFERENCES repository_observations(path, revision)
) STRICT;
CREATE TABLE operation_events (
    id INTEGER PRIMARY KEY,
    path BLOB NOT NULL,
    revision INTEGER,
    at TEXT NOT NULL,
    kind TEXT NOT NULL CHECK(kind IN ('stage', 'unstage', 'discard', 'commit', 'push', 'workspace', 'pull_fetch')),
    record TEXT NOT NULL CHECK(json_valid(record)),
    FOREIGN KEY(path, revision) REFERENCES repository_observations(path, revision)
) STRICT;
CREATE INDEX operation_repository ON operation_events(path, id DESC);
CREATE TABLE scan_reports (
    id INTEGER PRIMARY KEY,
    scanned_at TEXT NOT NULL,
    report TEXT NOT NULL CHECK(json_valid(report))
) STRICT;
CREATE TABLE scheduled_occurrences (
    plan TEXT NOT NULL,
    slot TEXT NOT NULL,
    claimed_at TEXT NOT NULL,
    PRIMARY KEY(plan, slot)
) STRICT;
"#;
