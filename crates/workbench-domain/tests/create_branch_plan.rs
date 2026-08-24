use workbench_domain::operations::create_branch::plan_create_branch_from_issue;
use workbench_domain::policy::github_flow_defaults;
use workbench_domain::repository::BranchState;

fn branch_state(name: &str) -> BranchState {
    BranchState {
        name: name.into(),
        head_oid: Some("abc".into()),
        upstream: Some("origin/main".into()),
        base_branch: Some("main".into()),
        ahead: 0,
        behind: 0,
        dirty_paths: vec![],
        is_protected: true,
    }
}

#[test]
fn plans_feature_branch_for_issue_42() {
    let policy = github_flow_defaults();
    let current = branch_state("main");

    let plan =
        plan_create_branch_from_issue(&policy, 42, "Add resumable uploads", &current).unwrap();

    assert!(plan.summary.contains("feature/42-add-resumable-uploads"));
    assert!(matches!(
        plan.risk,
        workbench_domain::operations::plan::RiskClass::Low
    ));

    let mut stable = plan.clone();
    stable.id = ulid::Ulid::nil();
    insta::assert_yaml_snapshot!("create_branch_issue_42", stable);
}

#[test]
fn rejects_issue_zero() {
    let policy = github_flow_defaults();
    let current = branch_state("main");

    let err = plan_create_branch_from_issue(&policy, 0, "Nope", &current).unwrap_err();

    assert!(matches!(
        err,
        workbench_domain::WorkbenchError::InvalidBranchName { ref reason }
            if reason == "issue number must be >= 1"
    ));
}
