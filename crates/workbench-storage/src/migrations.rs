use rusqlite::{Connection, OptionalExtension};

use workbench_application::error::AppError;

const MIGRATIONS: &[(&str, &str)] = &[
    ("001_initial", include_str!("migrations/001_initial.sql")),
    (
        "002_remote_tests",
        include_str!("migrations/002_remote_tests.sql"),
    ),
];

pub fn apply(conn: &Connection) -> Result<(), AppError> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS schema_migrations (
            id TEXT PRIMARY KEY,
            applied_at TEXT NOT NULL
        )",
        [],
    )
    .map_err(|e| AppError::Storage {
        detail: e.to_string(),
    })?;

    for (id, sql) in MIGRATIONS {
        let applied: bool = conn
            .query_row(
                "SELECT 1 FROM schema_migrations WHERE id = ?1",
                [*id],
                |_| Ok(()),
            )
            .optional()
            .map_err(|e| AppError::Storage {
                detail: e.to_string(),
            })?
            .is_some();

        if applied {
            continue;
        }

        let tx = conn
            .unchecked_transaction()
            .map_err(|e| AppError::Storage {
                detail: e.to_string(),
            })?;
        tx.execute_batch(sql).map_err(|e| AppError::Storage {
            detail: e.to_string(),
        })?;
        tx.execute(
            "INSERT INTO schema_migrations (id, applied_at) VALUES (?1, datetime('now'))",
            [*id],
        )
        .map_err(|e| AppError::Storage {
            detail: e.to_string(),
        })?;
        tx.commit().map_err(|e| AppError::Storage {
            detail: e.to_string(),
        })?;
    }

    Ok(())
}
