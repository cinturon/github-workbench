use workbench_domain::policy::{evaluate_current_branch_policy, github_flow_defaults, Severity};

#[test]
fn allowed_prefix_is_silent() {
    let policy = github_flow_defaults();
    let findings = evaluate_current_branch_policy(&policy, "feature/42-add-resumable-uploads");
    assert!(findings.is_empty());
}

#[test]
fn default_branch_is_silent() {
    let policy = github_flow_defaults();
    assert!(evaluate_current_branch_policy(&policy, "main").is_empty());
}

#[test]
fn unknown_prefix_is_warning() {
    let policy = github_flow_defaults();
    let findings = evaluate_current_branch_policy(&policy, "wip/experiment");
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].rule_id, "branches.allowed-prefixes");
    assert_eq!(findings[0].severity, Severity::Warning);
}
