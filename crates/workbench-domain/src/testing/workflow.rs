use std::collections::BTreeMap;

use serde::Serialize;

use super::{TestPlan, TestingError};

pub const RESULT_ARTIFACT_NAME: &str = "github-workbench-result";
pub const RESULT_MANIFEST_FILE: &str = "github-workbench-result.json";

pub fn remote_test_branch(prefix: &str, session_id: &str) -> Result<String, TestingError> {
    validate_identifier(prefix, "branch prefix")?;
    validate_identifier(session_id, "session id")?;
    Ok(format!("{prefix}/{session_id}"))
}

pub fn workflow_file_path(session_id: &str) -> Result<String, TestingError> {
    validate_identifier(session_id, "session id")?;
    Ok(format!(
        ".github/workflows/github-workbench-test-{session_id}.yml"
    ))
}

pub fn generate_workflow(
    plan: &TestPlan,
    session_id: &str,
    branch_name: &str,
) -> Result<String, TestingError> {
    validate_identifier(session_id, "session id")?;
    validate_identifier(branch_name, "branch name")?;

    let action_uses = if plan.action_path == "." {
        "./".to_string()
    } else {
        format!("./{}", plan.action_path.trim_start_matches("./"))
    };

    let manifest_script = format!(
        "cat > \"$RUNNER_TEMP/{RESULT_MANIFEST_FILE}\" <<'JSON'\n\
         {{\n\
         \u{20}\u{20}\"schema_version\": 1,\n\
         \u{20}\u{20}\"session_id\": \"{session_id}\",\n\
         \u{20}\u{20}\"case\": \"{}\",\n\
         \u{20}\u{20}\"runner\": \"ubuntu-latest\",\n\
         \u{20}\u{20}\"action_outcome\": \"${{{{ steps.action-under-test.outcome }}}}\",\n\
         \u{20}\u{20}\"outputs\": {{}}\n\
         }}\n\
         JSON",
        plan.name
    );

    let steps = vec![
        WorkflowStep {
            name: "Checkout test branch".into(),
            id: None,
            uses: Some("actions/checkout@v4".into()),
            run: None,
            shell: None,
            continue_on_error: None,
            if_condition: None,
            with: BTreeMap::new(),
            env: BTreeMap::new(),
        },
        WorkflowStep {
            name: "Run action under test".into(),
            id: Some("action-under-test".into()),
            uses: Some(action_uses),
            run: None,
            shell: None,
            continue_on_error: Some(true),
            if_condition: None,
            with: plan.inputs.clone(),
            env: plan.environment.clone(),
        },
        WorkflowStep {
            name: "Write result manifest".into(),
            id: None,
            uses: None,
            run: Some(manifest_script),
            shell: Some("bash".into()),
            continue_on_error: None,
            if_condition: Some("always()".into()),
            with: BTreeMap::new(),
            env: BTreeMap::new(),
        },
        WorkflowStep {
            name: "Upload result manifest".into(),
            id: None,
            uses: Some("actions/upload-artifact@v4".into()),
            run: None,
            shell: None,
            continue_on_error: None,
            if_condition: Some("always()".into()),
            with: BTreeMap::from([
                ("name".into(), RESULT_ARTIFACT_NAME.into()),
                (
                    "path".into(),
                    format!("${{{{ runner.temp }}}}/{RESULT_MANIFEST_FILE}"),
                ),
                ("if-no-files-found".into(), "error".into()),
            ]),
            env: BTreeMap::new(),
        },
        WorkflowStep {
            name: "Propagate action outcome".into(),
            id: None,
            uses: None,
            run: Some("test \"${{ steps.action-under-test.outcome }}\" = success".into()),
            shell: Some("bash".into()),
            continue_on_error: None,
            if_condition: Some("always()".into()),
            with: BTreeMap::new(),
            env: BTreeMap::new(),
        },
    ];

    let document = WorkflowDocument {
        name: format!("GitHub Workbench Test {session_id}"),
        trigger: WorkflowTrigger {
            push: PushTrigger {
                branches: vec![branch_name.to_string()],
            },
        },
        permissions: Permissions {
            contents: "read".into(),
        },
        concurrency: Concurrency {
            group: format!("github-workbench-test-{session_id}"),
            cancel_in_progress: false,
        },
        jobs: BTreeMap::from([(
            "test".into(),
            WorkflowJob {
                runs_on: plan.runner.clone(),
                timeout_minutes: plan.timeout_minutes,
                steps,
            },
        )]),
    };

    serde_yaml::to_string(&document).map_err(|error| TestingError::WorkflowGeneration {
        detail: error.to_string(),
    })
}

fn validate_identifier(value: &str, description: &str) -> Result<(), TestingError> {
    let invalid = value.is_empty()
        || value.starts_with('/')
        || value.ends_with('/')
        || value.contains("..")
        || value.contains('\\')
        || value
            .chars()
            .any(|character| character.is_control() || character.is_whitespace());
    if invalid {
        return Err(TestingError::WorkflowGeneration {
            detail: format!("invalid {description}: {value}"),
        });
    }
    Ok(())
}

#[derive(Serialize)]
struct WorkflowDocument {
    name: String,
    #[serde(rename = "on")]
    trigger: WorkflowTrigger,
    permissions: Permissions,
    concurrency: Concurrency,
    jobs: BTreeMap<String, WorkflowJob>,
}

#[derive(Serialize)]
struct WorkflowTrigger {
    push: PushTrigger,
}

#[derive(Serialize)]
struct PushTrigger {
    branches: Vec<String>,
}

#[derive(Serialize)]
struct Permissions {
    contents: String,
}

#[derive(Serialize)]
#[serde(rename_all = "kebab-case")]
struct Concurrency {
    group: String,
    cancel_in_progress: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "kebab-case")]
struct WorkflowJob {
    runs_on: String,
    timeout_minutes: u16,
    steps: Vec<WorkflowStep>,
}

#[derive(Serialize)]
#[serde(rename_all = "kebab-case")]
struct WorkflowStep {
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    uses: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    run: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    shell: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    continue_on_error: Option<bool>,
    #[serde(rename = "if", skip_serializing_if = "Option::is_none")]
    if_condition: Option<String>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    with: BTreeMap<String, String>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    env: BTreeMap<String, String>,
}
