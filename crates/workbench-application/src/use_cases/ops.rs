use std::path::Path;

use crate::error::AppError;
use crate::ports::{GitClient, OperationRecord, OperationStore};

pub fn list_project_operations<G, S>(
    git: &G,
    store: &S,
    path: &Path,
    limit: Option<u32>,
) -> Result<Vec<OperationRecord>, AppError>
where
    G: GitClient,
    S: OperationStore,
{
    let root = git.resolve_toplevel(path)?;
    let project = store
        .get_project_by_path(&root)?
        .ok_or(AppError::RepositoryNotMapped)?;
    store.list_operations(&project.id, limit.unwrap_or(20))
}
