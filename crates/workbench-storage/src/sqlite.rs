use std::path::Path;
use std::sync::Mutex;

use rusqlite::{params, Connection, OptionalExtension};
use workbench_application::action_tests::TestSessionStatus;
use workbench_application::error::AppError;
use workbench_application::ports::{
    CleanupItemRecord, NewCleanupItem, NewProject, NewTestSession, OperationRecord, OperationStore,
    ProjectRecord, StepRecord, TestSessionRecord, TestSessionStore, TestSessionUpdate,
};
use workbench_domain::operations::plan::{OperationPlan, StepStatus};
use workbench_domain::repository::RepositorySnapshot;

use crate::migrations;

pub struct SqliteStore {
    conn: Mutex<Connection>,
}

impl SqliteStore {
    pub fn open(path: &Path) -> Result<Self, AppError> {
        let conn = Connection::open(path).map_err(|e| AppError::Storage {
            detail: e.to_string(),
        })?;
        migrations::apply(&conn)?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    fn lock_conn(&self) -> Result<std::sync::MutexGuard<'_, Connection>, AppError> {
        self.conn.lock().map_err(|e| AppError::Storage {
            detail: e.to_string(),
        })
    }
}

fn step_status_to_text(status: StepStatus) -> Result<String, AppError> {
    serde_json::to_string(&status).map_err(|e| AppError::Storage {
        detail: e.to_string(),
    })
}

fn step_status_from_text(text: &str) -> Result<StepStatus, AppError> {
    serde_json::from_str(text).map_err(|e| AppError::Storage {
        detail: e.to_string(),
    })
}

fn test_session_status_to_text(status: TestSessionStatus) -> Result<String, AppError> {
    serde_json::to_string(&status).map_err(|e| AppError::Storage {
        detail: e.to_string(),
    })
}

fn test_session_status_from_text(text: &str) -> Result<TestSessionStatus, AppError> {
    serde_json::from_str(text).map_err(|e| AppError::Storage {
        detail: e.to_string(),
    })
}

fn run_id_to_i64(run_id: Option<u64>) -> Result<Option<i64>, AppError> {
    run_id
        .map(i64::try_from)
        .transpose()
        .map_err(|e| AppError::Storage {
            detail: e.to_string(),
        })
}

fn run_id_from_i64(run_id: Option<i64>) -> Result<Option<u64>, AppError> {
    run_id
        .map(u64::try_from)
        .transpose()
        .map_err(|e| AppError::Storage {
            detail: e.to_string(),
        })
}

fn row_to_test_session(row: &rusqlite::Row<'_>) -> Result<TestSessionRecord, rusqlite::Error> {
    let status_text: String = row.get(7)?;
    let status = test_session_status_from_text(&status_text).map_err(|e| {
        rusqlite::Error::FromSqlConversionFailure(7, rusqlite::types::Type::Text, Box::new(e))
    })?;
    let run_id = run_id_from_i64(row.get(6)?).map_err(|e| {
        rusqlite::Error::FromSqlConversionFailure(6, rusqlite::types::Type::Integer, Box::new(e))
    })?;
    Ok(TestSessionRecord {
        id: row.get(0)?,
        project_id: row.get(1)?,
        session_id: row.get(2)?,
        commit_sha: row.get(3)?,
        remote_ref: row.get(4)?,
        workflow_name: row.get(5)?,
        run_id,
        status,
        result_json: row.get(8)?,
        evidence_dir: row.get(9)?,
        created_at: row.get(10)?,
        updated_at: row.get(11)?,
    })
}

fn row_to_cleanup_item(row: &rusqlite::Row<'_>) -> Result<CleanupItemRecord, rusqlite::Error> {
    Ok(CleanupItemRecord {
        id: row.get(0)?,
        project_id: row.get(1)?,
        resource_kind: row.get(2)?,
        resource_id: row.get(3)?,
        expected_identity: row.get(4)?,
        due_at: row.get(5)?,
        status: row.get(6)?,
        created_at: row.get(7)?,
        updated_at: row.get(8)?,
    })
}

fn row_to_project(row: &rusqlite::Row<'_>) -> Result<ProjectRecord, rusqlite::Error> {
    Ok(ProjectRecord {
        id: row.get(0)?,
        local_path: row.get(1)?,
        github_host: row.get(2)?,
        owner: row.get(3)?,
        repo: row.get(4)?,
        remote_name: row.get(5)?,
        created_at: row.get(6)?,
        updated_at: row.get(7)?,
    })
}

fn row_to_step(row: &rusqlite::Row<'_>) -> Result<StepRecord, rusqlite::Error> {
    let status_text: String = row.get(4)?;
    let status = step_status_from_text(&status_text).map_err(|e| {
        rusqlite::Error::FromSqlConversionFailure(4, rusqlite::types::Type::Text, Box::new(e))
    })?;
    Ok(StepRecord {
        id: row.get(0)?,
        operation_id: row.get(1)?,
        sequence: row.get(2)?,
        kind: row.get(3)?,
        status,
        detail_json: row.get(5)?,
        output_text: row.get(6)?,
    })
}

impl OperationStore for SqliteStore {
    fn upsert_project(&self, project: NewProject<'_>) -> Result<ProjectRecord, AppError> {
        let conn = self.lock_conn()?;

        if let Some(existing) = conn
            .query_row(
                "SELECT id, local_path, github_host, owner, repo, remote_name, created_at, updated_at
                 FROM projects WHERE local_path = ?1",
                [project.local_path],
                row_to_project,
            )
            .optional()
            .map_err(|e| AppError::Storage {
                detail: e.to_string(),
            })?
        {
            conn.execute(
                "UPDATE projects
                 SET github_host = ?1, owner = ?2, repo = ?3, remote_name = ?4, updated_at = ?5
                 WHERE local_path = ?6",
                params![
                    project.github_host,
                    project.owner,
                    project.repo,
                    project.remote_name,
                    project.now,
                    project.local_path,
                ],
            )
            .map_err(|e| AppError::Storage {
                detail: e.to_string(),
            })?;

            return Ok(ProjectRecord {
                id: existing.id,
                local_path: existing.local_path,
                github_host: project.github_host.map(str::to_string),
                owner: project.owner.map(str::to_string),
                repo: project.repo.map(str::to_string),
                remote_name: project.remote_name.map(str::to_string),
                created_at: existing.created_at,
                updated_at: project.now.to_string(),
            });
        }

        conn.execute(
            "INSERT INTO projects (id, local_path, github_host, owner, repo, remote_name, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                project.id,
                project.local_path,
                project.github_host,
                project.owner,
                project.repo,
                project.remote_name,
                project.now,
                project.now,
            ],
        )
        .map_err(|e| AppError::Storage {
            detail: e.to_string(),
        })?;

        Ok(ProjectRecord {
            id: project.id.to_string(),
            local_path: project.local_path.to_string(),
            github_host: project.github_host.map(str::to_string),
            owner: project.owner.map(str::to_string),
            repo: project.repo.map(str::to_string),
            remote_name: project.remote_name.map(str::to_string),
            created_at: project.now.to_string(),
            updated_at: project.now.to_string(),
        })
    }

    fn get_project_by_path(&self, path: &Path) -> Result<Option<ProjectRecord>, AppError> {
        let key = path.to_string_lossy();
        let conn = self.lock_conn()?;
        conn.query_row(
            "SELECT id, local_path, github_host, owner, repo, remote_name, created_at, updated_at
             FROM projects WHERE local_path = ?1",
            [key.as_ref()],
            row_to_project,
        )
        .optional()
        .map_err(|e| AppError::Storage {
            detail: e.to_string(),
        })
    }

    fn create_operation(
        &self,
        project_id: &str,
        id: &str,
        kind: &str,
        status: &str,
        plan: &OperationPlan,
        snapshot: &RepositorySnapshot,
        started_at: &str,
    ) -> Result<OperationRecord, AppError> {
        let plan_json = serde_json::to_string(plan).map_err(|e| AppError::Storage {
            detail: e.to_string(),
        })?;
        let snapshot_json = serde_json::to_string(snapshot).map_err(|e| AppError::Storage {
            detail: e.to_string(),
        })?;

        let conn = self.lock_conn()?;
        conn.execute(
            "INSERT INTO operations (id, project_id, kind, status, plan_json, snapshot_json, started_at, completed_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, NULL)",
            params![
                id,
                project_id,
                kind,
                status,
                plan_json,
                snapshot_json,
                started_at,
            ],
        )
        .map_err(|e| AppError::Storage {
            detail: e.to_string(),
        })?;

        Ok(OperationRecord {
            id: id.to_string(),
            project_id: project_id.to_string(),
            kind: kind.to_string(),
            status: status.to_string(),
            plan_json,
            started_at: Some(started_at.to_string()),
            completed_at: None,
            snapshot_json: Some(snapshot_json),
            steps: vec![],
        })
    }

    fn update_operation(
        &self,
        id: &str,
        status: &str,
        completed_at: Option<&str>,
    ) -> Result<(), AppError> {
        let conn = self.lock_conn()?;
        let updated = conn
            .execute(
                "UPDATE operations SET status = ?1, completed_at = ?2 WHERE id = ?3",
                params![status, completed_at, id],
            )
            .map_err(|e| AppError::Storage {
                detail: e.to_string(),
            })?;
        if updated == 0 {
            return Err(AppError::Storage {
                detail: format!("missing operation {id}"),
            });
        }
        Ok(())
    }

    fn append_step(
        &self,
        operation_id: &str,
        id: &str,
        sequence: i32,
        kind: &str,
        status: StepStatus,
        detail_json: Option<&str>,
        now: &str,
    ) -> Result<StepRecord, AppError> {
        let status_text = step_status_to_text(status)?;
        let conn = self.lock_conn()?;
        conn.execute(
            "INSERT INTO operation_steps (id, operation_id, sequence, kind, status, detail_json, output_text, started_at, completed_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, NULL, ?7, NULL)",
            params![
                id,
                operation_id,
                sequence,
                kind,
                status_text,
                detail_json,
                now,
            ],
        )
        .map_err(|e| AppError::Storage {
            detail: e.to_string(),
        })?;

        Ok(StepRecord {
            id: id.to_string(),
            operation_id: operation_id.to_string(),
            sequence,
            kind: kind.to_string(),
            status,
            detail_json: detail_json.map(str::to_string),
            output_text: None,
        })
    }

    fn update_step(
        &self,
        id: &str,
        status: StepStatus,
        output_text: Option<&str>,
        completed_at: Option<&str>,
    ) -> Result<(), AppError> {
        let status_text = step_status_to_text(status)?;
        let conn = self.lock_conn()?;
        let updated = conn
            .execute(
                "UPDATE operation_steps SET status = ?1, output_text = ?2, completed_at = ?3 WHERE id = ?4",
                params![status_text, output_text, completed_at, id],
            )
            .map_err(|e| AppError::Storage {
                detail: e.to_string(),
            })?;
        if updated == 0 {
            return Err(AppError::Storage {
                detail: format!("missing step {id}"),
            });
        }
        Ok(())
    }

    fn list_operations(
        &self,
        project_id: &str,
        limit: u32,
    ) -> Result<Vec<OperationRecord>, AppError> {
        let conn = self.lock_conn()?;
        let mut stmt = conn
            .prepare(
                "SELECT id, project_id, kind, status, plan_json, snapshot_json, started_at, completed_at
                 FROM operations
                 WHERE project_id = ?1
                 ORDER BY started_at DESC
                 LIMIT ?2",
            )
            .map_err(|e| AppError::Storage {
                detail: e.to_string(),
            })?;

        let operations = stmt
            .query_map(params![project_id, limit], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, Option<String>>(5)?,
                    row.get::<_, Option<String>>(6)?,
                    row.get::<_, Option<String>>(7)?,
                ))
            })
            .map_err(|e| AppError::Storage {
                detail: e.to_string(),
            })?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| AppError::Storage {
                detail: e.to_string(),
            })?;

        let mut records = Vec::with_capacity(operations.len());
        for (id, proj_id, kind, status, plan_json, snapshot_json, started_at, completed_at) in
            operations
        {
            let mut step_stmt = conn
                .prepare(
                    "SELECT id, operation_id, sequence, kind, status, detail_json, output_text
                     FROM operation_steps
                     WHERE operation_id = ?1
                     ORDER BY sequence ASC",
                )
                .map_err(|e| AppError::Storage {
                    detail: e.to_string(),
                })?;

            let steps = step_stmt
                .query_map([&id], row_to_step)
                .map_err(|e| AppError::Storage {
                    detail: e.to_string(),
                })?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| AppError::Storage {
                    detail: e.to_string(),
                })?;

            records.push(OperationRecord {
                id,
                project_id: proj_id,
                kind,
                status,
                plan_json,
                snapshot_json,
                started_at,
                completed_at,
                steps,
            });
        }

        Ok(records)
    }
}

impl TestSessionStore for SqliteStore {
    fn create_test_session(
        &self,
        session: NewTestSession<'_>,
    ) -> Result<TestSessionRecord, AppError> {
        let status_text = test_session_status_to_text(session.status.clone())?;
        let conn = self.lock_conn()?;
        conn.execute(
            "INSERT INTO test_sessions
             (id, project_id, session_key, commit_sha, remote_ref, workflow_name,
              run_id, status, result_json, evidence_dir, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, NULL, ?7, ?8, NULL, ?9, ?9)",
            params![
                session.id,
                session.project_id,
                session.session_id,
                session.commit_sha,
                session.remote_ref,
                session.workflow_name,
                status_text,
                session.result_json,
                session.now,
            ],
        )
        .map_err(|e| AppError::Storage {
            detail: e.to_string(),
        })?;

        Ok(TestSessionRecord {
            id: session.id.to_string(),
            project_id: session.project_id.to_string(),
            session_id: session.session_id.to_string(),
            commit_sha: session.commit_sha.to_string(),
            remote_ref: session.remote_ref.to_string(),
            workflow_name: session.workflow_name.to_string(),
            run_id: None,
            status: session.status,
            result_json: session.result_json.to_string(),
            evidence_dir: None,
            created_at: session.now.to_string(),
            updated_at: session.now.to_string(),
        })
    }

    fn update_test_session(&self, update: TestSessionUpdate<'_>) -> Result<(), AppError> {
        let status_text = test_session_status_to_text(update.status)?;
        let run_id = run_id_to_i64(update.run_id)?;
        let conn = self.lock_conn()?;
        let updated = conn
            .execute(
                "UPDATE test_sessions
                 SET run_id = ?1, status = ?2, result_json = ?3,
                     evidence_dir = ?4, updated_at = ?5
                 WHERE project_id = ?6 AND session_key = ?7",
                params![
                    run_id,
                    status_text,
                    update.result_json,
                    update.evidence_dir,
                    update.now,
                    update.project_id,
                    update.session_id,
                ],
            )
            .map_err(|e| AppError::Storage {
                detail: e.to_string(),
            })?;
        if updated == 0 {
            return Err(AppError::Storage {
                detail: format!(
                    "missing test session {}/{}",
                    update.project_id, update.session_id
                ),
            });
        }
        Ok(())
    }

    fn get_test_session(
        &self,
        project_id: &str,
        session_id: &str,
    ) -> Result<Option<TestSessionRecord>, AppError> {
        let conn = self.lock_conn()?;
        conn.query_row(
            "SELECT id, project_id, session_key, commit_sha, remote_ref,
                    workflow_name, run_id, status, result_json, evidence_dir,
                    created_at, updated_at
             FROM test_sessions
             WHERE project_id = ?1 AND session_key = ?2",
            params![project_id, session_id],
            row_to_test_session,
        )
        .optional()
        .map_err(|e| AppError::Storage {
            detail: e.to_string(),
        })
    }

    fn list_test_sessions(
        &self,
        project_id: &str,
        limit: u32,
    ) -> Result<Vec<TestSessionRecord>, AppError> {
        let conn = self.lock_conn()?;
        let mut stmt = conn
            .prepare(
                "SELECT id, project_id, session_key, commit_sha, remote_ref,
                        workflow_name, run_id, status, result_json, evidence_dir,
                        created_at, updated_at
                 FROM test_sessions
                 WHERE project_id = ?1
                 ORDER BY updated_at DESC
                 LIMIT ?2",
            )
            .map_err(|e| AppError::Storage {
                detail: e.to_string(),
            })?;

        let rows = stmt
            .query_map(params![project_id, limit], row_to_test_session)
            .map_err(|e| AppError::Storage {
                detail: e.to_string(),
            })?;

        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| AppError::Storage {
                detail: e.to_string(),
            })
    }

    fn enqueue_cleanup(&self, item: NewCleanupItem<'_>) -> Result<CleanupItemRecord, AppError> {
        let conn = self.lock_conn()?;
        conn.execute(
            "INSERT INTO cleanup_items
             (id, project_id, resource_kind, resource_id, expected_identity,
              due_at, status, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'pending', ?7, ?7)",
            params![
                item.id,
                item.project_id,
                item.resource_kind,
                item.resource_id,
                item.expected_identity,
                item.due_at,
                item.now,
            ],
        )
        .map_err(|e| AppError::Storage {
            detail: e.to_string(),
        })?;

        Ok(CleanupItemRecord {
            id: item.id.to_string(),
            project_id: item.project_id.to_string(),
            resource_kind: item.resource_kind.to_string(),
            resource_id: item.resource_id.to_string(),
            expected_identity: item.expected_identity.to_string(),
            due_at: item.due_at.to_string(),
            status: "pending".to_string(),
            created_at: item.now.to_string(),
            updated_at: item.now.to_string(),
        })
    }

    fn get_cleanup_item(
        &self,
        project_id: &str,
        item_id: &str,
    ) -> Result<Option<CleanupItemRecord>, AppError> {
        let conn = self.lock_conn()?;
        conn.query_row(
            "SELECT id, project_id, resource_kind, resource_id, expected_identity,
                    due_at, status, created_at, updated_at
             FROM cleanup_items
             WHERE project_id = ?1 AND id = ?2",
            params![project_id, item_id],
            row_to_cleanup_item,
        )
        .optional()
        .map_err(|e| AppError::Storage {
            detail: e.to_string(),
        })
    }

    fn list_cleanup_items(&self, project_id: &str) -> Result<Vec<CleanupItemRecord>, AppError> {
        let conn = self.lock_conn()?;
        let mut stmt = conn
            .prepare(
                "SELECT id, project_id, resource_kind, resource_id, expected_identity,
                        due_at, status, created_at, updated_at
                 FROM cleanup_items
                 WHERE project_id = ?1
                 ORDER BY due_at ASC",
            )
            .map_err(|e| AppError::Storage {
                detail: e.to_string(),
            })?;

        let rows = stmt
            .query_map([project_id], row_to_cleanup_item)
            .map_err(|e| AppError::Storage {
                detail: e.to_string(),
            })?;

        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| AppError::Storage {
                detail: e.to_string(),
            })
    }

    fn complete_cleanup_item(&self, item_id: &str, now: &str) -> Result<(), AppError> {
        let conn = self.lock_conn()?;
        let updated = conn
            .execute(
                "UPDATE cleanup_items
                 SET status = 'completed', updated_at = ?1
                 WHERE id = ?2 AND status = 'pending'",
                params![now, item_id],
            )
            .map_err(|e| AppError::Storage {
                detail: e.to_string(),
            })?;
        if updated == 0 {
            return Err(AppError::Storage {
                detail: format!("missing cleanup item {item_id}"),
            });
        }
        Ok(())
    }
}
