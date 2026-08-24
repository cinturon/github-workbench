use workbench_domain::policy::{github_flow_defaults, parse_policy_yaml};
use workbench_domain::WorkbenchError;

#[test]
fn parses_minimal_github_flow_yaml() {
    let yaml = r#"
schema-version: 1
strategy:
  preset: github-flow
  default-branch: main
"#;
    let cfg = parse_policy_yaml(yaml).unwrap();
    assert_eq!(cfg.schema_version, 1);
    assert_eq!(cfg.strategy.default_branch, "main");
    assert_eq!(cfg.branches.feature.pattern, "feature/{issue}-{slug}");
}

#[test]
fn unknown_field_is_error() {
    let yaml = r#"
schema-version: 1
strategy:
  preset: github-flow
  default-branch: main
typo-field: true
"#;
    let err = parse_policy_yaml(yaml).unwrap_err();
    match err {
        WorkbenchError::InvalidPolicy { findings } => {
            assert!(findings.iter().any(|f| f.rule_id.contains("unknown")));
        }
        other => panic!("unexpected {other:?}"),
    }
}

#[test]
fn defaults_preset_fills_branch_patterns() {
    let cfg = github_flow_defaults();
    assert!(cfg.branches.feature.require_issue);
    assert_eq!(cfg.pull_requests.required_base, "main");
    assert!(cfg.pull_requests.draft_by_default);
}
