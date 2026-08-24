use workbench_domain::testing::{
    generate_workflow, normalize_test_case, parse_action_definition,
    parse_test_case_yaml, remote_test_branch, workflow_file_path,
    RESULT_ARTIFACT_NAME,
};

#[test]
fn generates_the_locked_single_job_workflow() {
    let action = parse_action_definition(
        "action.yml",
        "name: Smoke\nruns:\n  using: composite\n  steps: []\n",
    )
    .unwrap();
    let case = parse_test_case_yaml(
        r#"
schema-version: 1
name: smoke-composite
action:
  path: .
runner:
  os: [ubuntu-latest]
  timeout-minutes: 10
permissions:
  contents: read
inputs:
  mode: smoke
environment:
  REPORT_LEVEL: summary
expect:
  conclusion: success
"#,
    )
    .unwrap();
    let plan = normalize_test_case(case, &action, 15).unwrap();
    let session = "01JABCDEF0123456789ABCDEFG";
    let branch = remote_test_branch("github-workbench/test", session).unwrap();
    let workflow = generate_workflow(&plan, session, &branch).unwrap();

    assert_eq!(
        branch,
        "github-workbench/test/01JABCDEF0123456789ABCDEFG"
    );
    assert_eq!(
        workflow_file_path(session).unwrap(),
        ".github/workflows/github-workbench-test-01JABCDEF0123456789ABCDEFG.yml"
    );
    assert_eq!(RESULT_ARTIFACT_NAME, "github-workbench-result");
    assert!(workflow.contains("runs-on: ubuntu-latest"));
    assert!(workflow.contains("actions/upload-artifact@v4"));
    assert!(workflow.contains("continue-on-error: true"));
    assert!(workflow.contains("permissions:\n  contents: read"));
    assert!(!workflow.contains("\nenvironment:"));
    insta::assert_snapshot!("minimal_remote_test", workflow);
}
