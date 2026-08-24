use workbench_domain::policy::{github_flow_defaults, parse_policy_yaml, RetentionHours};

#[test]
fn phase_three_defaults_are_safe() {
    let policy = github_flow_defaults();

    assert_eq!(policy.remote_testing.isolation, "ephemeral-branch");
    assert_eq!(policy.remote_testing.branch_prefix, "github-workbench/test");
    assert_eq!(policy.remote_testing.max_matrix_jobs, 6);
    assert_eq!(policy.remote_testing.default_timeout_minutes, 15);
    assert_eq!(
        policy.remote_testing.successful_ref_retention,
        RetentionHours(0)
    );
    assert_eq!(
        policy.remote_testing.failed_ref_retention,
        RetentionHours(72)
    );
}

#[test]
fn parses_exact_remote_testing_keys() {
    let policy = parse_policy_yaml(
        r#"
schema-version: 1
strategy:
  preset: github-flow
  default-branch: main
remote-testing:
  isolation: ephemeral-branch
  branch-prefix: workbench/check
  max-matrix-jobs: 1
  default-timeout-minutes: 20
  successful-ref-retention: 1h
  failed-ref-retention: 96h
"#,
    )
    .unwrap();

    assert_eq!(policy.remote_testing.branch_prefix, "workbench/check");
    assert_eq!(policy.remote_testing.max_matrix_jobs, 1);
    assert_eq!(policy.remote_testing.default_timeout_minutes, 20);
    assert_eq!(
        policy.remote_testing.failed_ref_retention,
        RetentionHours(96)
    );
}

#[test]
fn rejects_unsafe_branch_prefixes_and_zero_timeout() {
    for yaml in [
        "branch-prefix: ../main\ndefault-timeout-minutes: 15",
        "branch-prefix: github-workbench/test\ndefault-timeout-minutes: 0",
    ] {
        let document = format!(
            "schema-version: 1\nstrategy:\n  preset: github-flow\n  default-branch: main\nremote-testing:\n  {}",
            yaml.replace('\n', "\n  ")
        );
        assert!(parse_policy_yaml(&document).is_err());
    }
}
