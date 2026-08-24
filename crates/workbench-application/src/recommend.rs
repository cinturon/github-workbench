use workbench_domain::policy::PolicyConfig;
use workbench_domain::repository::{BranchState, RepositorySnapshot};

pub fn recommend_next_action(
    policy: &PolicyConfig,
    snapshot: &RepositorySnapshot,
    branch: &BranchState,
) -> String {
    if snapshot.detached_head {
        return "Check out a branch before starting work (for example git checkout main).".into();
    }
    let on_default = branch.name == policy.strategy.default_branch;
    if !snapshot.dirty_paths.is_empty() {
        if on_default {
            return "Commit or stash local changes, then start an issue branch with gww issue start <n> --title <text>.".into();
        }
        return "Commit your changes with Git, then run gww push --plan.".into();
    }
    if on_default {
        return "Start a policy-compliant feature branch: gww issue start <n> --title <text>."
            .into();
    }
    if branch.ahead > 0 {
        return "Preview and push this branch: gww push --plan.".into();
    }
    "Nothing to push. Commit new work, or create a draft pull request (Phase 3+).".into()
}
