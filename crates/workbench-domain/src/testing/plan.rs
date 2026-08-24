use std::collections::BTreeMap;
use std::path::{Component, Path};

use serde::{Deserialize, Serialize};

use super::{
    ActionDefinition, ActionRuntime, TestCase, TestPermissions, TestingError,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TestPlan {
    pub name: String,
    pub description: Option<String>,
    pub action_path: String,
    pub runner: String,
    pub timeout_minutes: u16,
    pub permissions: TestPermissions,
    pub inputs: BTreeMap<String, String>,
    pub environment: BTreeMap<String, String>,
    pub assertions: TestAssertions,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TestAssertions {
    pub conclusion: String,
    pub log_contains: Vec<String>,
    pub log_not_contains: Vec<String>,
}

pub fn normalize_test_case(
    case: TestCase,
    action: &ActionDefinition,
    default_timeout_minutes: u16,
) -> Result<TestPlan, TestingError> {
    if case.schema_version != 1 {
        return invalid(format!(
            "schema-version must be 1, found {}",
            case.schema_version
        ));
    }

    let using = match &action.runtime {
        ActionRuntime::Composite => None,
        ActionRuntime::Unsupported { using } => Some(using.clone()),
    };
    if let Some(using) = using {
        return Err(TestingError::ActionNotComposite { using });
    }

    validate_name(&case.name)?;
    validate_relative_path(&case.action.path)?;

    if case.runner.os.as_slice() != ["ubuntu-latest"] {
        return invalid("runner.os must contain only ubuntu-latest".into());
    }

    let timeout_minutes =
        case.runner.timeout_minutes.unwrap_or(default_timeout_minutes);
    if timeout_minutes == 0 {
        return invalid("runner.timeout-minutes must be greater than zero".into());
    }

    if case.permissions.contents != "read" {
        return invalid("permissions must be exactly contents: read".into());
    }

    for key in case.inputs.keys().chain(case.environment.keys()) {
        let upper = key.to_ascii_uppercase();
        if ["SECRET", "TOKEN", "PASSWORD"]
            .iter()
            .any(|needle| upper.contains(needle))
        {
            return Err(TestingError::SecretLikeKey { key: key.clone() });
        }
    }

    let allowed_conclusions = ["success", "failure"];
    if !allowed_conclusions.contains(&case.expect.conclusion.as_str()) {
        return invalid(format!(
            "expect.conclusion must be one of {}",
            allowed_conclusions.join(", ")
        ));
    }

    let mut log_contains = Vec::new();
    let mut log_not_contains = Vec::new();
    for expectation in case.expect.logs {
        match (expectation.contains, expectation.not_contains) {
            (Some(value), None) => log_contains.push(value),
            (None, Some(value)) => log_not_contains.push(value),
            _ => {
                return invalid(
                    "each expect.logs item must contain exactly one matcher"
                        .into(),
                )
            }
        }
    }

    Ok(TestPlan {
        name: case.name,
        description: case.description,
        action_path: case.action.path,
        runner: "ubuntu-latest".into(),
        timeout_minutes,
        permissions: case.permissions,
        inputs: case.inputs,
        environment: case.environment,
        assertions: TestAssertions {
            conclusion: case.expect.conclusion,
            log_contains,
            log_not_contains,
        },
    })
}

fn validate_name(name: &str) -> Result<(), TestingError> {
    if name.is_empty()
        || !name
            .chars()
            .all(|character| character.is_ascii_alphanumeric()
                || matches!(character, '-' | '_' | '.'))
    {
        return invalid(
            "name must contain only ASCII letters, digits, dash, underscore, or dot"
                .into(),
        );
    }
    Ok(())
}

fn validate_relative_path(value: &str) -> Result<(), TestingError> {
    let path = Path::new(value);
    if path.is_absolute()
        || path.components().any(|component| {
            matches!(
                component,
                Component::ParentDir
                    | Component::RootDir
                    | Component::Prefix(_)
            )
        })
    {
        return invalid("action.path must stay inside the repository".into());
    }
    Ok(())
}

fn invalid<T>(detail: String) -> Result<T, TestingError> {
    Err(TestingError::InvalidTestCase { detail })
}
