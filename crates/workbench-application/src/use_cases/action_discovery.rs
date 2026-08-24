use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use workbench_domain::testing::{
    parse_action_definition, parse_test_case_yaml, ActionDefinition, ActionRuntime,
};

use crate::AppError;

const SKIPPED_DIRECTORIES: &[&str] = &[".git", "target", "node_modules", "dist", "build"];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiscoveredAction {
    pub definition: ActionDefinition,
    pub supported: bool,
    pub warning: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiscoveredTestCase {
    pub path: PathBuf,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActionTestCatalog {
    pub actions: Vec<DiscoveredAction>,
    pub tests: Vec<DiscoveredTestCase>,
}

pub fn discover_action_tests(repo_root: &Path) -> Result<ActionTestCatalog, AppError> {
    let mut files = Vec::new();
    collect_files(repo_root, &mut files)?;
    files.sort();

    let mut actions = Vec::new();
    let mut tests = Vec::new();
    for path in files {
        let relative = path
            .strip_prefix(repo_root)
            .map_err(|error| AppError::Io {
                path: path.display().to_string(),
                detail: error.to_string(),
            })?
            .to_path_buf();

        if is_action_manifest(&relative) {
            let yaml = read_text(&path)?;
            let manifest_path = relative.to_string_lossy().into_owned();
            let definition = parse_action_definition(&manifest_path, &yaml).map_err(|error| {
                AppError::TestCaseInvalid {
                    path: manifest_path.clone(),
                    detail: error.to_string(),
                }
            })?;
            let (supported, warning) = match &definition.runtime {
                ActionRuntime::Composite => (true, None),
                ActionRuntime::Unsupported { using } => (
                    false,
                    Some(format!(
                        "Action runtime `{using}` is unsupported; only composite actions can be tested."
                    )),
                ),
            };
            actions.push(DiscoveredAction {
                definition,
                supported,
                warning,
            });
        } else if is_test_case(&relative) {
            let yaml = read_text(&path)?;
            let relative_display = relative.to_string_lossy().into_owned();
            let case = parse_test_case_yaml(&yaml).map_err(|error| AppError::TestCaseInvalid {
                path: relative_display,
                detail: error.to_string(),
            })?;
            tests.push(DiscoveredTestCase {
                path: relative,
                name: case.name,
            });
        }
    }

    Ok(ActionTestCatalog { actions, tests })
}

fn collect_files(directory: &Path, files: &mut Vec<PathBuf>) -> Result<(), AppError> {
    let entries = fs::read_dir(directory).map_err(|error| io_error(directory, error))?;
    for entry in entries {
        let entry = entry.map_err(|error| io_error(directory, error))?;
        let path = entry.path();
        let file_type = entry.file_type().map_err(|error| io_error(&path, error))?;
        if file_type.is_dir() {
            let name = entry.file_name();
            if SKIPPED_DIRECTORIES
                .iter()
                .any(|skipped| name.to_str() == Some(*skipped))
            {
                continue;
            }
            collect_files(&path, files)?;
        } else if file_type.is_file() {
            files.push(path);
        }
    }
    Ok(())
}

fn is_action_manifest(path: &Path) -> bool {
    matches!(
        path.file_name().and_then(|name| name.to_str()),
        Some("action.yml" | "action.yaml")
    )
}

fn is_test_case(path: &Path) -> bool {
    path.parent() == Some(Path::new(".github-workbench/tests"))
        && path.extension().and_then(|extension| extension.to_str()) == Some("yml")
}

fn read_text(path: &Path) -> Result<String, AppError> {
    fs::read_to_string(path).map_err(|error| io_error(path, error))
}

fn io_error(path: &Path, error: std::io::Error) -> AppError {
    AppError::Io {
        path: path.display().to_string(),
        detail: error.to_string(),
    }
}
