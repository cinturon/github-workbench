use super::plan::{GitCommand, OperationPlan, RiskClass};
use crate::policy::PolicyConfig;
use crate::repository::BranchState;
use crate::workflow::naming::branch_name;
use crate::WorkbenchError;
use ulid::Ulid;

pub fn plan_create_branch_from_issue(
    policy: &PolicyConfig,
    issue: u64,
    title: &str,
    current: &BranchState,
) -> Result<OperationPlan, WorkbenchError> {
    if issue == 0 {
        return Err(WorkbenchError::InvalidBranchName {
            reason: "issue number must be >= 1".into(),
        });
    }

    let feature = &policy.branches.feature;
    let name = branch_name(&feature.pattern, issue, title)?;
    if name == policy.strategy.default_branch {
        return Err(WorkbenchError::ProtectedBranchMisuse { branch: name });
    }

    let start_point = feature.start_from.clone();
    let mut commands = Vec::new();
    let mut rationale = vec![
        format!("Feature branches follow pattern `{}`.", feature.pattern),
        format!(
            "Feature branches require an issue: {}.",
            feature.require_issue
        ),
        format!("Feature branches start from `{start_point}`."),
    ];

    if current.name != start_point {
        rationale.push(format!(
            "Current branch `{}` is not `{start_point}`; fetch the base before creating the branch.",
            current.name
        ));
        commands.push(GitCommand::Fetch {
            remote: "origin".into(),
        });
    }

    commands.push(GitCommand::CreateBranch {
        name: name.clone(),
        start_point: start_point.clone(),
    });

    Ok(OperationPlan {
        id: Ulid::new(),
        kind: "create-branch-from-issue".into(),
        risk: RiskClass::Low,
        summary: format!("Create branch {name} from {start_point} for issue #{issue}"),
        rationale,
        commands,
        preconditions: vec![format!("Branch `{start_point}` exists or can be fetched.")],
        findings: vec![],
    })
}
