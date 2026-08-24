CREATE TABLE projects (
    id TEXT PRIMARY KEY,
    local_path TEXT NOT NULL UNIQUE,
    github_host TEXT,
    owner TEXT,
    repo TEXT,
    remote_name TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE operations (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES projects(id),
    kind TEXT NOT NULL,
    status TEXT NOT NULL,
    plan_json TEXT NOT NULL,
    snapshot_json TEXT,
    started_at TEXT,
    completed_at TEXT
);

CREATE TABLE operation_steps (
    id TEXT PRIMARY KEY,
    operation_id TEXT NOT NULL REFERENCES operations(id),
    sequence INTEGER NOT NULL,
    kind TEXT NOT NULL,
    status TEXT NOT NULL,
    detail_json TEXT,
    output_text TEXT,
    started_at TEXT,
    completed_at TEXT
);

CREATE INDEX idx_operations_project_started ON operations(project_id, started_at DESC);
CREATE INDEX idx_steps_operation_sequence ON operation_steps(operation_id, sequence);
