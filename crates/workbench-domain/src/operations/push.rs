use super::plan::{GitCommand, OperationPlan, RiskClass};
use crate::policy::PolicyConfig;
use crate::repository::BranchState;
use crate::WorkbenchError;
use ulid::Ulid;

pub fn plan_push(
    policy: &PolicyConfig,
    current: &BranchState,
    remote: &str,
) -> Result<OperationPlan, WorkbenchError> {
    if current.name == policy.strategy.default_branch {
        return Err(WorkbenchError::ProtectedBranchMisuse {
            branch: current.name.clone(),
        });
    }

    if current.ahead == 0 {
        return Ok(OperationPlan {
            id: Ulid::new(),
            kind: "push".into(),
            risk: RiskClass::Low,
            summary: format!(
                "Nothing to push: `{}` has no commits ahead of its comparison ref.",
                current.name
            ),
            rationale: vec![
                "Phase 2 pushes only commits that already exist on the current feature branch."
                    .into(),
                format!("Ahead count is {}.", current.ahead),
            ],
            commands: vec![],
            preconditions: vec!["Working tree is clean.".into()],
            findings: vec![],
        });
    }

    let set_upstream = current.upstream.is_none();
    let risk = if set_upstream {
        RiskClass::Low
    } else {
        RiskClass::Medium
    };
    let upstream_reason = match &current.upstream {
        None => {
            "No upstream is set; the push will create the remote branch and set upstream.".into()
        }
        Some(upstream) => format!(
            "Upstream is `{upstream}`; this updates the existing remote feature branch."
        ),
    };

    Ok(OperationPlan {
        id: Ulid::new(),
        kind: "push".into(),
        risk,
        summary: format!("Push `{}` to `{}/{}`", current.name, remote, current.name),
        rationale: vec![
            format!("Remote `{remote}` is the selected push target."),
            upstream_reason,
            "Force push is never used.".into(),
        ],
        commands: vec![
            GitCommand::Fetch {
                remote: remote.into(),
            },
            GitCommand::PushRef {
                remote: remote.into(),
                local_ref: current.name.clone(),
                remote_ref: current.name.clone(),
                set_upstream,
            },
        ],
        preconditions: vec![
            "Working tree is clean.".into(),
            format!("Local branch `{}` exists.", current.name),
            format!("Remote `{remote}` is reachable."),
        ],
        findings: vec![],
    })
}
