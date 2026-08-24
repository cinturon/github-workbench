use std::path::{Path, PathBuf};

use crate::action_tests::{RemoteTestResult, StoredSessionState};
use crate::ports::{GitClient, OperationStore, ProjectRecord, TestSessionRecord, TestSessionStore};
use crate::AppError;

pub fn list_sessions<G, S>(
    git: &G,
    store: &S,
    path: &Path,
) -> Result<Vec<TestSessionRecord>, AppError>
where
    G: GitClient,
    S: OperationStore + TestSessionStore,
{
    let (_, project) = resolve_project(git, store, path)?;
    store.list_test_sessions(&project.id, u32::MAX)
}

pub fn get_session_result<G, S>(
    git: &G,
    store: &S,
    path: &Path,
    session_id: &str,
) -> Result<Option<RemoteTestResult>, AppError>
where
    G: GitClient,
    S: OperationStore + TestSessionStore,
{
    let (_, project) = resolve_project(git, store, path)?;
    let Some(session) = store.get_test_session(&project.id, session_id)? else {
        return Ok(None);
    };
    let state: StoredSessionState =
        serde_json::from_str(&session.result_json).map_err(|error| AppError::Storage {
            detail: format!("could not deserialize remote test session `{session_id}`: {error}"),
        })?;
    Ok(state.result)
}

pub(crate) fn resolve_project<G, S>(
    git: &G,
    store: &S,
    path: &Path,
) -> Result<(PathBuf, ProjectRecord), AppError>
where
    G: GitClient,
    S: OperationStore,
{
    let root = git.resolve_toplevel(path)?;
    let project = store
        .get_project_by_path(&root)?
        .ok_or(AppError::RepositoryNotMapped)?;
    Ok((root, project))
}
