use std::path::Path;

use workbench_domain::policy::PolicyConfig;
use workbench_domain::repository::{parse_github_remote, RepositorySnapshot};

use crate::error::AppError;
use crate::policy_source::load_policy;
use crate::ports::{
    Clock, GitClient, IdGenerator, NewProject, OperationStore, PolicySource, ProjectRecord,
};
use crate::remote::resolve_remote;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenOutcome {
    pub snapshot: RepositorySnapshot,
    pub project: ProjectRecord,
    pub policy: PolicyConfig,
    pub policy_source: &'static str,
}

pub fn open_repository<G, S, P, C, I>(
    git: &G,
    store: &S,
    policy: &P,
    clock: &C,
    ids: &I,
    path: &Path,
    remote_flag: Option<&str>,
) -> Result<OpenOutcome, AppError>
where
    G: GitClient,
    S: OperationStore,
    P: PolicySource,
    C: Clock,
    I: IdGenerator,
{
    let root = git.resolve_toplevel(path)?;
    let mut snapshot = git.snapshot(&root)?;
    let (policy, policy_source) = load_policy(policy, &root)?;
    let remote_name = resolve_remote(&snapshot.remotes, None, remote_flag)?;
    snapshot.selected_remote = Some(remote_name.clone());

    let identity = snapshot
        .remotes
        .iter()
        .find(|remote| remote.name == remote_name)
        .and_then(|remote| parse_github_remote(&remote.url));
    let id = ids.next().to_string();
    let local_path = root.to_string_lossy().into_owned();
    let now = clock.now_rfc3339();
    let project = store.upsert_project(NewProject {
        id: &id,
        local_path: &local_path,
        github_host: identity.as_ref().map(|value| value.host.as_str()),
        owner: identity.as_ref().map(|value| value.owner.as_str()),
        repo: identity.as_ref().map(|value| value.name.as_str()),
        remote_name: Some(&remote_name),
        now: &now,
    })?;

    Ok(OpenOutcome {
        snapshot,
        project,
        policy,
        policy_source,
    })
}
