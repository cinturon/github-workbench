use workbench_domain::operations::plan::{GitCommand, RiskClass};
use workbench_domain::operations::push::plan_push;
use workbench_domain::policy::github_flow_defaults;
use workbench_domain::repository::BranchState;
use workbench_domain::WorkbenchError;

fn feature_branch(ahead: u64, upstream: Option<&str>) -> BranchState {
    BranchState {
        name: "feature/42-add-resumable-uploads".into(),
        head_oid: Some("abc".into()),
        upstream: upstream.map(str::to_string),
        base_branch: Some("main".into()),
        ahead,
        behind: 0,
        dirty_paths: vec![],
        is_protected: false,
    }
}

#[test]
fn plans_low_risk_push_for_new_upstream() {
    let policy = github_flow_defaults();
    let current = feature_branch(2, None);
    let plan = plan_push(&policy, &current, "github").unwrap();
    assert_eq!(plan.kind, "push");
    assert_eq!(plan.risk, RiskClass::Low);
    assert!(plan.summary.contains("github"));
    assert!(matches!(
        &plan.commands[..],
        [
            GitCommand::Fetch { remote },
            GitCommand::PushRef {
                remote: push_remote,
                local_ref,
                remote_ref,
                set_upstream: true,
            }
        ] if remote == "github"
            && push_remote == "github"
            && local_ref == "feature/42-add-resumable-uploads"
            && remote_ref == "feature/42-add-resumable-uploads"
    ));
    let mut stable = plan.clone();
    stable.id = ulid::Ulid::nil();
    insta::assert_yaml_snapshot!("push_new_feature_branch", stable);
}

#[test]
fn nothing_to_push_when_ahead_is_zero() {
    let policy = github_flow_defaults();
    let plan = plan_push(
        &policy,
        &feature_branch(0, Some("github/feature/42-add-resumable-uploads")),
        "github",
    )
    .unwrap();
    assert!(plan.commands.is_empty());
    assert!(plan.summary.contains("Nothing to push"));
}

#[test]
fn refuses_push_of_default_branch() {
    let policy = github_flow_defaults();
    let current = BranchState {
        name: "main".into(),
        head_oid: Some("abc".into()),
        upstream: Some("github/main".into()),
        base_branch: Some("main".into()),
        ahead: 1,
        behind: 0,
        dirty_paths: vec![],
        is_protected: true,
    };
    let err = plan_push(&policy, &current, "github").unwrap_err();
    assert!(matches!(
        err,
        WorkbenchError::ProtectedBranchMisuse { branch } if branch == "main"
    ));
}

#[test]
fn existing_upstream_is_medium_risk() {
    let policy = github_flow_defaults();
    let plan = plan_push(
        &policy,
        &feature_branch(1, Some("github/feature/42-add-resumable-uploads")),
        "github",
    )
    .unwrap();
    assert_eq!(plan.risk, RiskClass::Medium);
    assert!(matches!(
        plan.commands.last(),
        Some(GitCommand::PushRef {
            set_upstream: false,
            ..
        })
    ));
}
