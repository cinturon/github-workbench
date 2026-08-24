use std::path::Path;

use workbench_domain::operations::create_branch::plan_create_branch_from_issue;
use workbench_domain::operations::plan::OperationPlan;
use workbench_domain::policy::PolicyConfig;
use workbench_domain::repository::{parse_github_remote, RepositorySnapshot};

use crate::error::AppError;
use crate::executor::{execute_plan, ExecuteOutcome};
use crate::policy_source::load_policy;
use crate::ports::{Clock, GitClient, IdGenerator, NewProject, OperationStore, PolicySource};
use crate::remote::resolve_remote;

#[allow(clippy::too_many_arguments)]
pub fn plan_start_issue<G, S, P>(
    git: &G,
    store: &S,
    policy_source: &P,
    path: &Path,
    issue: u64,
    title: &str,
    remote_flag: Option<&str>,
) -> Result<(OperationPlan, RepositorySnapshot, PolicyConfig), AppError>
where
    G: GitClient,
    S: OperationStore,
    P: PolicySource,
{
    let root = git.resolve_toplevel(path)?;
    let mut snapshot = git.snapshot(&root)?;
    let (policy, _) = load_policy(policy_source, &root)?;
    let mapped_remote = store
        .get_project_by_path(&root)?
        .and_then(|project| project.remote_name);
    let remote = resolve_remote(&snapshot.remotes, mapped_remote.as_deref(), remote_flag)?;
    snapshot.selected_remote = Some(remote.clone());
    let branch = git.branch_state(&root, &policy.branches.feature.start_from)?;
    let plan = plan_create_branch_from_issue(&policy, issue, title, &branch, &remote)?;

    Ok((plan, snapshot, policy))
}

#[allow(clippy::too_many_arguments)]
pub fn execute_start_issue<G, S, P, C, I>(
    git: &G,
    store: &S,
    policy_source: &P,
    clock: &C,
    ids: &I,
    path: &Path,
    issue: u64,
    title: &str,
    remote_flag: Option<&str>,
) -> Result<ExecuteOutcome, AppError>
where
    G: GitClient,
    S: OperationStore,
    P: PolicySource,
    C: Clock,
    I: IdGenerator,
{
    let (plan, snapshot, _) =
        plan_start_issue(git, store, policy_source, path, issue, title, remote_flag)?;
    let remote_name = snapshot
        .selected_remote
        .as_deref()
        .ok_or(AppError::RepositoryNotMapped)?;
    let identity = snapshot
        .remotes
        .iter()
        .find(|remote| remote.name == remote_name)
        .and_then(|remote| parse_github_remote(&remote.url));
    let project_id = ids.next().to_string();
    let now = clock.now_rfc3339();
    let project = store.upsert_project(NewProject {
        id: &project_id,
        local_path: &snapshot.root,
        github_host: identity.as_ref().map(|value| value.host.as_str()),
        owner: identity.as_ref().map(|value| value.owner.as_str()),
        repo: identity.as_ref().map(|value| value.name.as_str()),
        remote_name: Some(remote_name),
        now: &now,
    })?;

    execute_plan(git, store, clock, ids, &project.id, &snapshot, &plan)
}
