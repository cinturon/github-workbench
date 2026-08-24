use workbench_domain::testing::{parse_action_definition, ActionRuntime, TestingError};

#[test]
fn parses_a_composite_action_without_interpreting_steps() {
    let action = parse_action_definition(
        "action.yml",
        r#"
name: Upload report
description: Uploads a generated report
inputs:
  report-path:
    description: Report path
    required: true
runs:
  using: composite
  steps:
    - shell: bash
      run: echo "Upload completed"
"#,
    )
    .unwrap();

    assert_eq!(action.manifest_path, "action.yml");
    assert_eq!(action.name, "Upload report");
    assert_eq!(action.runtime, ActionRuntime::Composite);
    assert!(action.inputs["report-path"].required);
}

#[test]
fn preserves_an_unsupported_runtime_for_discovery_warnings() {
    let action = parse_action_definition(
        "tools/action.yml",
        r#"
name: JavaScript action
runs:
  using: node20
  main: index.js
"#,
    )
    .unwrap();

    assert_eq!(
        action.runtime,
        ActionRuntime::Unsupported {
            using: "node20".into()
        }
    );
}

#[test]
fn reports_invalid_action_yaml_structurally() {
    let error = parse_action_definition("action.yml", "name: missing-runs").unwrap_err();

    assert!(matches!(
        error,
        TestingError::InvalidAction {
            ref manifest_path,
            ..
        } if manifest_path == "action.yml"
    ));
}
