use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::TestingError;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct TestCase {
    pub schema_version: u32,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    pub action: TestAction,
    pub runner: TestRunner,
    #[serde(default)]
    pub permissions: TestPermissions,
    #[serde(default)]
    pub inputs: BTreeMap<String, String>,
    #[serde(default)]
    pub environment: BTreeMap<String, String>,
    pub expect: TestExpectation,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TestAction {
    pub path: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct TestRunner {
    pub os: Vec<String>,
    #[serde(default)]
    pub timeout_minutes: Option<u16>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TestPermissions {
    #[serde(default = "read")]
    pub contents: String,
}

impl Default for TestPermissions {
    fn default() -> Self {
        Self {
            contents: read(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TestExpectation {
    pub conclusion: String,
    #[serde(default)]
    pub logs: Vec<LogExpectation>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct LogExpectation {
    #[serde(default)]
    pub contains: Option<String>,
    #[serde(default)]
    pub not_contains: Option<String>,
}

pub fn parse_test_case_yaml(yaml: &str) -> Result<TestCase, TestingError> {
    serde_yaml::from_str(yaml).map_err(|error| TestingError::InvalidTestCase {
        detail: error.to_string(),
    })
}

fn read() -> String {
    "read".into()
}
