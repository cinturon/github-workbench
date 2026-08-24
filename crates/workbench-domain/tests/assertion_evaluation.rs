use workbench_domain::testing::{evaluate_assertions, TestAssertions};

fn assertions() -> TestAssertions {
    TestAssertions {
        conclusion: "success".into(),
        log_contains: vec!["Upload completed".into()],
        log_not_contains: vec!["secret=".into()],
    }
}

const MANIFEST: &str = r#"{
  "schema_version": 1,
  "session_id": "01JABC",
  "case": "smoke-composite",
  "runner": "ubuntu-latest",
  "action_outcome": "success",
  "outputs": {}
}"#;

#[test]
fn passes_when_conclusion_manifest_and_logs_match() {
    let report = evaluate_assertions(
        &assertions(),
        "success",
        Some(MANIFEST),
        "starting\nUpload completed\n",
        "https://github.com/acme/widgets/actions/runs/7",
    );

    assert!(report.passed);
    assert!(report.failures.is_empty());
}

#[test]
fn reports_all_independent_failures() {
    let report = evaluate_assertions(
        &assertions(),
        "failure",
        Some(&MANIFEST.replace(
            "\"action_outcome\": \"success\"",
            "\"action_outcome\": \"failure\"",
        )),
        "secret=value\n",
        "https://github.com/acme/widgets/actions/runs/7",
    );

    assert!(!report.passed);
    assert_eq!(report.failures.len(), 4);
    assert!(report
        .failures
        .iter()
        .any(|failure| failure.rule == "run.conclusion"));
    assert!(report
        .failures
        .iter()
        .any(|failure| failure.rule == "manifest.action-outcome"));
    assert!(report
        .failures
        .iter()
        .any(|failure| failure.rule == "logs.contains"));
    assert!(report
        .failures
        .iter()
        .any(|failure| failure.rule == "logs.not-contains"));
}

#[test]
fn missing_manifest_is_a_failed_required_assertion() {
    let report = evaluate_assertions(
        &assertions(),
        "success",
        None,
        "Upload completed",
        "https://github.com/acme/widgets/actions/runs/7",
    );

    assert!(!report.passed);
    assert!(report.failures[0]
        .remediation
        .contains("https://github.com/acme/widgets/actions/runs/7"));
}
