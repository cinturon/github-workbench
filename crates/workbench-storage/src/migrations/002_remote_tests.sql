CREATE TABLE test_sessions (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES projects(id),
    session_key TEXT NOT NULL,
    commit_sha TEXT NOT NULL,
    remote_ref TEXT NOT NULL,
    workflow_name TEXT NOT NULL,
    run_id INTEGER,
    status TEXT NOT NULL,
    result_json TEXT NOT NULL,
    evidence_dir TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    UNIQUE(project_id, session_key)
);

CREATE TABLE cleanup_items (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES projects(id),
    resource_kind TEXT NOT NULL,
    resource_id TEXT NOT NULL,
    expected_identity TEXT NOT NULL,
    due_at TEXT NOT NULL,
    status TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE INDEX idx_test_sessions_project_updated
    ON test_sessions(project_id, updated_at DESC);

CREATE INDEX idx_cleanup_items_project_status_due
    ON cleanup_items(project_id, status, due_at);
