use std::path::Path;

use workbench_domain::policy::{evaluate_current_branch_policy, PolicyConfig, PolicyFinding};
use workbench_domain::repository::{BranchState, RepositorySnapshot};

use crate::error::AppError;
use crate::policy_source::load_policy;
use crate::ports::{GitClient, PolicySource};
use crate::recommend::recommend_next_action;
use crate::remote::resolve_remote;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatusOutcome {
    pub snapshot: RepositorySnapshot,
    pub branch: BranchState,
    pub policy: PolicyConfig,
    pub policy_source: &'static str,
    pub findings: Vec<PolicyFinding>,
    pub recommended_next_action: String,
}

pub fn repository_status<G, P>(
    git: &G,
    policy: &P,
    path: &Path,
    mapped_remote: Option<&str>,
    remote_flag: Option<&str>,
) -> Result<StatusOutcome, AppError>
where
    G: GitClient,
    P: PolicySource,
{
    let root = git.resolve_toplevel(path)?;
    let mut snapshot = git.snapshot(&root)?;
    let (policy, policy_source) = load_policy(policy, &root)?;
    let comparison_ref = snapshot
        .upstream
        .as_deref()
        .unwrap_or(&policy.strategy.default_branch);
    let branch = git.branch_state(&root, comparison_ref)?;

    match resolve_remote(&snapshot.remotes, mapped_remote, remote_flag) {
        Ok(remote_name) => snapshot.selected_remote = Some(remote_name),
        Err(AppError::RemoteNotResolved { .. } | AppError::RepositoryNotMapped) => {
            snapshot.selected_remote = None;
        }
        Err(error) => return Err(error),
    }

    let findings = evaluate_current_branch_policy(&policy, &branch.name);
    let recommended_next_action = recommend_next_action(&policy, &snapshot, &branch);

    Ok(StatusOutcome {
        snapshot,
        branch,
        policy,
        policy_source,
        findings,
        recommended_next_action,
    })
}
