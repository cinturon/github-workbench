use workbench_domain::policy::{
    evaluate_commit_message_policy, github_flow_defaults, Enforcement, PolicyFinding, Severity,
};

#[test]
fn conventional_commit_violation_uses_configured_severity() {
    let mut policy = github_flow_defaults();

    policy.commits.conventional_commits = Enforcement::Warning;
    let warnings = evaluate_commit_message_policy(&policy, "update parser");
    assert_eq!(warnings.len(), 1);
    assert_eq!(warnings[0].severity, Severity::Warning);

    policy.commits.conventional_commits = Enforcement::Blocker;
    let blockers = evaluate_commit_message_policy(&policy, "update parser");
    assert_eq!(blockers.len(), 1);
    assert_eq!(blockers[0].severity, Severity::Blocker);
}

#[test]
fn conventional_commit_policy_accepts_valid_messages_and_respects_off() {
    let mut policy = github_flow_defaults();
    assert!(
        evaluate_commit_message_policy(&policy, "fix(parser): reject invalid input").is_empty()
    );

    policy.commits.conventional_commits = Enforcement::Off;
    assert!(evaluate_commit_message_policy(&policy, "update parser").is_empty());
}

#[test]
fn policy_explanation_matches_golden_yaml() {
    let mut policy = github_flow_defaults();
    policy.commits.conventional_commits = Enforcement::Warning;
    let findings: Vec<PolicyFinding> = evaluate_commit_message_policy(&policy, "update parser");

    insta::assert_yaml_snapshot!("commit_message_policy_explanation", findings);
}
