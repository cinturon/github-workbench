use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::TestAssertions;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResultManifest {
    pub schema_version: u32,
    pub session_id: String,
    #[serde(rename = "case")]
    pub case_name: String,
    pub runner: String,
    pub action_outcome: String,
    pub outputs: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssertionFailure {
    pub rule: String,
    pub expected: String,
    pub actual: String,
    pub remediation: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssertionReport {
    pub passed: bool,
    pub manifest: Option<ResultManifest>,
    pub failures: Vec<AssertionFailure>,
}

pub fn evaluate_assertions(
    assertions: &TestAssertions,
    run_conclusion: &str,
    manifest_json: Option<&str>,
    logs: &str,
    run_url: &str,
) -> AssertionReport {
    let mut failures = Vec::new();

    if run_conclusion != assertions.conclusion {
        failures.push(AssertionFailure {
            rule: "run.conclusion".into(),
            expected: assertions.conclusion.clone(),
            actual: run_conclusion.into(),
            remediation: format!("Inspect the completed run at {run_url}."),
        });
    }

    let manifest = match manifest_json {
        Some(json) => match serde_json::from_str::<ResultManifest>(json) {
            Ok(manifest) if manifest.schema_version == 1 => Some(manifest),
            Ok(manifest) => {
                failures.push(AssertionFailure {
                    rule: "manifest.schema-version".into(),
                    expected: "1".into(),
                    actual: manifest.schema_version.to_string(),
                    remediation: format!(
                        "Inspect the uploaded result artifact and run at {run_url}."
                    ),
                });
                Some(manifest)
            }
            Err(error) => {
                failures.push(AssertionFailure {
                    rule: "manifest.valid-json".into(),
                    expected: "valid result manifest JSON".into(),
                    actual: error.to_string(),
                    remediation: format!(
                        "Inspect the uploaded result artifact and run at {run_url}."
                    ),
                });
                None
            }
        },
        None => {
            failures.push(AssertionFailure {
                rule: "manifest.required".into(),
                expected: "github-workbench-result artifact".into(),
                actual: "manifest missing".into(),
                remediation: format!("Open {run_url} and inspect the artifact-upload step."),
            });
            None
        }
    };

    if let Some(manifest) = &manifest {
        if manifest.action_outcome != assertions.conclusion {
            failures.push(AssertionFailure {
                rule: "manifest.action-outcome".into(),
                expected: assertions.conclusion.clone(),
                actual: manifest.action_outcome.clone(),
                remediation: format!("Inspect the action and manifest steps at {run_url}."),
            });
        }
    }

    for needle in &assertions.log_contains {
        if !logs.contains(needle) {
            failures.push(AssertionFailure {
                rule: "logs.contains".into(),
                expected: needle.clone(),
                actual: "substring absent".into(),
                remediation: format!("Inspect downloaded logs or open {run_url}."),
            });
        }
    }

    for needle in &assertions.log_not_contains {
        if logs.contains(needle) {
            failures.push(AssertionFailure {
                rule: "logs.not-contains".into(),
                expected: format!("absence of {needle}"),
                actual: "substring present".into(),
                remediation: format!("Inspect downloaded logs or open {run_url}."),
            });
        }
    }

    AssertionReport {
        passed: failures.is_empty(),
        manifest,
        failures,
    }
}
