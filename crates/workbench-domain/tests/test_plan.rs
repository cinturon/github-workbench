use workbench_domain::testing::{
    normalize_test_case, parse_action_definition, parse_test_case_yaml, TestingError,
};

const MINIMAL: &str = r#"
schema-version: 1
name: smoke-composite
description: Optional one-line description.
action:
  path: .
runner:
  os:
    - ubuntu-latest
permissions:
  contents: read
inputs: {}
environment: {}
expect:
  conclusion: success
  logs:
    - contains: Upload completed
    - not-contains: secret=
"#;

#[test]
fn normalizes_the_minimal_test_and_policy_timeout() {
    let action = parse_action_definition(
        "action.yml",
        "name: Smoke\nruns:\n  using: composite\n  steps: []\n",
    )
    .unwrap();
    let case = parse_test_case_yaml(MINIMAL).unwrap();
    let plan = normalize_test_case(case, &action, 15).unwrap();

    assert_eq!(plan.name, "smoke-composite");
    assert_eq!(plan.action_path, ".");
    assert_eq!(plan.runner, "ubuntu-latest");
    assert_eq!(plan.timeout_minutes, 15);
    assert_eq!(plan.permissions.contents, "read");
    assert_eq!(plan.assertions.conclusion, "success");
    assert_eq!(plan.assertions.log_contains, vec!["Upload completed"]);
    assert_eq!(plan.assertions.log_not_contains, vec!["secret="]);
}

#[test]
fn rejects_non_composite_actions() {
    let action = parse_action_definition(
        "action.yml",
        "name: Node\nruns:\n  using: node20\n  main: index.js\n",
    )
    .unwrap();

    let error =
        normalize_test_case(parse_test_case_yaml(MINIMAL).unwrap(), &action, 15).unwrap_err();

    assert!(matches!(
        error,
        TestingError::ActionNotComposite { ref using }
            if using == "node20"
    ));
}

#[test]
fn rejects_secret_looking_keys_before_remote_mutation() {
    let yaml = MINIMAL.replace("environment: {}", "environment:\n  DEPLOY_TOKEN: value");
    let action = parse_action_definition(
        "action.yml",
        "name: Smoke\nruns:\n  using: composite\n  steps: []\n",
    )
    .unwrap();

    let error = normalize_test_case(parse_test_case_yaml(&yaml).unwrap(), &action, 15).unwrap_err();

    assert!(matches!(
        error,
        TestingError::SecretLikeKey { ref key }
            if key == "DEPLOY_TOKEN"
    ));
}

#[test]
fn rejects_unknown_fields_and_non_ubuntu_runners() {
    assert!(parse_test_case_yaml(&MINIMAL.replace("inputs: {}", "inputz: {}")).is_err());

    let action = parse_action_definition(
        "action.yml",
        "name: Smoke\nruns:\n  using: composite\n  steps: []\n",
    )
    .unwrap();
    let windows = MINIMAL.replace("ubuntu-latest", "windows-latest");

    assert!(normalize_test_case(parse_test_case_yaml(&windows).unwrap(), &action, 15,).is_err());
}
