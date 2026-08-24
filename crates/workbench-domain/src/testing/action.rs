use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::TestingError;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActionDefinition {
    pub manifest_path: String,
    pub name: String,
    pub description: Option<String>,
    pub inputs: BTreeMap<String, ActionInput>,
    pub runtime: ActionRuntime,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActionInput {
    pub description: Option<String>,
    pub required: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum ActionRuntime {
    Composite,
    Unsupported { using: String },
}

#[derive(Debug, Deserialize)]
struct RawAction {
    name: String,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    inputs: BTreeMap<String, RawActionInput>,
    runs: RawRuns,
}

#[derive(Debug, Deserialize)]
struct RawActionInput {
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    required: bool,
}

#[derive(Debug, Deserialize)]
struct RawRuns {
    using: String,
}

pub fn parse_action_definition(
    manifest_path: &str,
    yaml: &str,
) -> Result<ActionDefinition, TestingError> {
    let raw: RawAction =
        serde_yaml::from_str(yaml).map_err(|error| TestingError::InvalidAction {
            manifest_path: manifest_path.to_string(),
            detail: error.to_string(),
        })?;

    let runtime = if raw.runs.using == "composite" {
        ActionRuntime::Composite
    } else {
        ActionRuntime::Unsupported {
            using: raw.runs.using,
        }
    };

    Ok(ActionDefinition {
        manifest_path: manifest_path.to_string(),
        name: raw.name,
        description: raw.description,
        inputs: raw
            .inputs
            .into_iter()
            .map(|(name, input)| {
                (
                    name,
                    ActionInput {
                        description: input.description,
                        required: input.required,
                    },
                )
            })
            .collect(),
        runtime,
    })
}
