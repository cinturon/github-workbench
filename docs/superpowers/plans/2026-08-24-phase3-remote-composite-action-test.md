# Phase 3 Remote Composite-Action Test Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Deliver an end-to-end remote test of one local composite GitHub Action on `ubuntu-latest`, exposed through the `gww` CLI and a thin Tauri Action Tests interface.

**Architecture:** Pure domain code parses actions and tests, generates deterministic workflows, and evaluates evidence. Application use cases coordinate Git, GitHub, policy, persistence, and cleanup exclusively through ports; process-based Git/`gh`, SQLite, CLI, and Tauri remain adapters or presentation layers.

**Tech Stack:** Rust 2021, clap 4, rusqlite, serde/serde_yaml/serde_json, thiserror, ulid, time, tempfile, insta, Tauri 2, Vite, React, TypeScript.

## Global Constraints

- **Prerequisite: merge Phase 2 before executing.** Phase 2 currently exists on `origin/cursor/phase2-implementation-plan-7895`, not `master`.
- Use Phase 2’s `OperationPlan`, `GitCommand::{Fetch, CreateBranch, PushRef}`, `RiskClass`, `StepStatus`, `GitClient`, `OperationStore`, `ProcessRunner`, `Clock`, `IdGenerator`, and `PolicySource`.
- `workbench-domain` must remain free of filesystem, Git, `gh`, SQLite, and Tauri dependencies.
- `workbench-application` must not depend directly on `workbench-git`, `workbench-storage`, or `workbench-github`; it consumes ports only.
- GitHub access must use a `gh` process adapter with program plus `Vec<String>` arguments. Never construct shell command strings.
- Never pass `--force`, `--force-with-lease`, or equivalent force refspecs.
- Remote tests support only composite actions on `ubuntu-latest`.
- Require a clean working tree before planning and again before executing a new remote test.
- The domain produces the normalized `TestPlan`, workflow YAML, workflow-path helper, and branch-name helper.
- The application produces an `OperationPlan` for create branch, commit generated paths, and push, plus a `RemoteTestSessionPlan`.
- `execute_remote_test` must authenticate, revalidate the clean snapshot, write the generated workflow, execute the Git plan, persist the session, correlate and poll the run, download evidence, evaluate assertions, update the session, and enqueue cleanup.
- `gww runs watch <session-id>` resumes the stored session without creating or pushing another branch.
- Branches use `<prefix>/<session-id>`, with policy prefix or default `github-workbench/test`.
- Workflow files use `.github/workflows/github-workbench-test-<session_id>.yml`.
- The required artifact name is `github-workbench-result`.
- Assertions are limited to expected conclusion and optional log `contains`/`not-contains`.
- A result manifest is required even when the workflow failed.
- Cleanup deletes only the recorded temporary remote ref after its current SHA matches the recorded expected SHA.
- SQLite stores evidence paths and redacted state only; never persist `GH_TOKEN`, `GITHUB_TOKEN`, raw credentials, or unredacted logs.
- Successful refs default to `0h` retention; failed refs default to `72h`.
- No timer-driven automatic cleanup is added in Phase 3.
- Exit codes are `0` success, `1` runtime/test failure, `2` invalid usage/configuration, `3` policy blocker, `4` authentication required, and `5` remote still pending.
- Default CI must never contact live `github.com`; GitHub adapter and orchestration tests use recorded or local fixtures.
- The optional live end-to-end procedure must use a disposable repository and run outside required CI.
- Keep existing Phase 2 CLI behavior unchanged.

---

## File Structure

```text
Cargo.toml
  Add shared dependencies and the Tauri workspace member.

.github/workflows/ci.yml
  Build/test Rust and desktop code without live GitHub access.

crates/workbench-domain/
  Cargo.toml
  src/error.rs
    Retain general domain errors.
  src/operations/plan.rs
    Add CommitPaths and DeleteRemoteRef commands.
  src/policy/schema.rs
    Add RemoteTestingConfig and retention-hour values.
  src/policy/preset.rs
    Supply Phase 3 remote-testing defaults.
  src/policy/load.rs
    Validate branch prefix, retention, and timeout.
  src/policy/mod.rs
    Export remote-testing policy types.
  src/testing/mod.rs
    Export the Phase 3 testing model.
  src/testing/error.rs
    Structured action/test/workflow parsing errors.
  src/testing/action.rs
    Parse action.yml/action.yaml without filesystem access.
  src/testing/case.rs
    Strictly parse the declarative test schema.
  src/testing/plan.rs
    Normalize tests and enforce runner, permissions, paths, and secret-key rules.
  src/testing/workflow.rs
    Build deterministic workflow YAML and naming helpers.
  src/testing/assertions.rs
    Parse result manifests and evaluate conclusion/log assertions.
  tests/action_definition.rs
  tests/test_plan.rs
  tests/workflow_generation.rs
  tests/assertion_evaluation.rs
  tests/remote_testing_policy.rs
  tests/operation_plan_remote_test.rs
  tests/snapshots/workflow_generation__minimal_remote_test.snap

crates/workbench-application/
  src/error.rs
    Add Phase 3 errors and exit-code mappings.
  src/ports.rs
    Extend GitClient and add GithubClient/TestSessionStore records.
  src/action_tests.rs
    Shared serializable session, result, and cleanup models.
  src/executor.rs
    Execute and journal the two new Git commands.
  src/clock.rs
    Add injectable sleeping for remote polling.
  src/fakes.rs
    Add deterministic GitHub/session fakes and extended Git behavior.
  src/lib.rs
    Export Phase 3 application types.
  src/use_cases/mod.rs
    Export Phase 3 use cases.
  src/use_cases/action_discovery.rs
    Discover action manifests and test definitions.
  src/use_cases/remote_test.rs
    Plan, execute, correlate, poll, download, and evaluate tests.
  src/use_cases/test_sessions.rs
    List, read, and resume stored sessions.
  src/use_cases/cleanup.rs
    Plan and execute exact-identity remote-ref cleanup.
  tests/support/mod.rs
    Shared deterministic remote-test harness.
  tests/remote_test_plan.rs
  tests/remote_test_execute.rs
  tests/remote_test_resume.rs
  tests/cleanup.rs

crates/workbench-storage/
  src/migrations.rs
    Register migration 002.
  src/migrations/002_remote_tests.sql
    Add test_sessions and cleanup_items.
  src/sqlite.rs
    Implement TestSessionStore.
  tests/remote_test_store.rs
    Verify migrations and persistence round trips.

crates/workbench-github/
  Cargo.toml
  src/lib.rs
    Export ProcessGithubClient.
  src/client.rs
    Execute allowlisted gh argv.
  src/env.rs
    Build a minimal gh environment.
  src/parser.rs
    Parse recorded run-list and run-detail JSON.
  tests/process_github_client.rs
  tests/fixtures/workflow_runs.json
  tests/fixtures/workflow_run_completed.json
  tests/fixtures/run_logs.txt

crates/workbench-git/
  src/argv.rs
    Render commit and delete-ref argv while forbidding force.
  src/client.rs
    Implement commit_paths, delete_remote_ref, and rev_parse.
  tests/git_integration.rs
    Exercise generated-only commits and exact remote deletion.

crates/workbench-cli/
  Cargo.toml
  src/args.rs
    Add action, runs, and cleanup subcommands.
  src/main.rs
    Wire concrete adapters to shared use cases.
  src/render.rs
    Render plans, sessions, results, and cleanup.
  tests/cli_remote_action.rs
    Exercise the CLI with local Git and a gh fixture program.

crates/workbench-desktop/
  package.json
  package-lock.json
  index.html
  tsconfig.json
  vite.config.ts
  src/main.tsx
  src/App.tsx
  src/api.ts
  src/styles.css
  src-tauri/Cargo.toml
  src-tauri/build.rs
  src-tauri/tauri.conf.json
  src-tauri/capabilities/default.json
  src-tauri/src/main.rs
  src-tauri/src/lib.rs
  src-tauri/src/commands.rs
  src-tauri/tests/action_tests_commands.rs
    Minimal Action Tests list/start/watch/result desktop shell.

README.md
  Document Phase 3 CLI and desktop entry points.

docs/architecture.md
  Record boundaries, execution flow, persistence, and cleanup invariants.

docs/superpowers/manual/phase3-live-e2e.md
  Document the opt-in disposable-repository live test.
```

## Locked Contract Examples

Minimal test file at `.github-workbench/tests/smoke-composite.yml`:

```yaml
schema-version: 1
name: smoke-composite
description: Optional one-line description.

action:
  path: .   # relative to repo root; must resolve to composite action.yml/yaml

runner:
  os:
    - ubuntu-latest
  timeout-minutes: 10   # optional; default from policy remote-testing or 15

permissions:
  contents: read

inputs: {}          # optional; all values strings
environment: {}     # optional; non-secret only

expect:
  conclusion: success
  logs:               # optional
    - contains: "Upload completed"
    - not-contains: "secret="
```

Required result manifest:

```json
{
  "schema_version": 1,
  "session_id": "01JABC...",
  "case": "smoke-composite",
  "runner": "ubuntu-latest",
  "action_outcome": "success",
  "outputs": {}
}
```

### Task 1: Parse Composite Action Definitions in the Domain

**Files:**
- Create: `crates/workbench-domain/src/testing/error.rs`
- Create: `crates/workbench-domain/src/testing/action.rs`
- Modify: `crates/workbench-domain/src/testing/mod.rs`
- Test: `crates/workbench-domain/tests/action_definition.rs`

**Interfaces:**
- Consumes: action manifest path and YAML as plain strings.
- Produces:
  - `parse_action_definition(manifest_path: &str, yaml: &str) -> Result<ActionDefinition, TestingError>`
  - `ActionDefinition`
  - `ActionRuntime::{Composite, Unsupported { using: String }}`
  - `ActionInput`

- [ ] **Step 1: Write the failing action parser tests**

```rust
use workbench_domain::testing::{
    parse_action_definition, ActionRuntime, TestingError,
};

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
    let error = parse_action_definition("action.yml", "name: missing-runs")
        .unwrap_err();

    assert!(matches!(
        error,
        TestingError::InvalidAction {
            ref manifest_path,
            ..
        } if manifest_path == "action.yml"
    ));
}
```

- [ ] **Step 2: Run the tests and verify the red state**

Run: `cargo test -p workbench-domain --test action_definition`

Expected: FAIL because `parse_action_definition`, `ActionRuntime`, and `TestingError` are not exported.

- [ ] **Step 3: Implement the parser and public model**

```rust
// crates/workbench-domain/src/testing/error.rs
use thiserror::Error;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum TestingError {
    #[error("invalid action manifest `{manifest_path}`: {detail}")]
    InvalidAction {
        manifest_path: String,
        detail: String,
    },

    #[error("invalid test case: {detail}")]
    InvalidTestCase { detail: String },

    #[error("could not generate remote-test workflow: {detail}")]
    WorkflowGeneration { detail: String },
}
```

```rust
// crates/workbench-domain/src/testing/action.rs
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
```

```rust
// crates/workbench-domain/src/testing/mod.rs
mod action;
mod error;

pub use action::{
    parse_action_definition, ActionDefinition, ActionInput, ActionRuntime,
};
pub use error::TestingError;
```

- [ ] **Step 4: Run the focused and domain test suites**

Run: `cargo test -p workbench-domain --test action_definition && cargo test -p workbench-domain`

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/workbench-domain/src/testing crates/workbench-domain/tests/action_definition.rs
git commit -m "feat(domain): parse action definitions"
```

### Task 2: Parse and Normalize Remote Test Cases

**Files:**
- Modify: `crates/workbench-domain/src/testing/error.rs`
- Create: `crates/workbench-domain/src/testing/case.rs`
- Create: `crates/workbench-domain/src/testing/plan.rs`
- Modify: `crates/workbench-domain/src/testing/mod.rs`
- Test: `crates/workbench-domain/tests/test_plan.rs`

**Interfaces:**
- Consumes:
  - `parse_action_definition(...)`
  - `ActionDefinition`
  - policy-provided default timeout.
- Produces:
  - `parse_test_case_yaml(yaml: &str) -> Result<TestCase, TestingError>`
  - `normalize_test_case(case: TestCase, action: &ActionDefinition, default_timeout_minutes: u16) -> Result<TestPlan, TestingError>`
  - `TestPlan`
  - `TestAssertions`
  - strict secret-key denylist for `SECRET`, `TOKEN`, and `PASSWORD`.

- [ ] **Step 1: Write failing normalization tests**

```rust
use workbench_domain::testing::{
    normalize_test_case, parse_action_definition, parse_test_case_yaml,
    TestingError,
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
        normalize_test_case(parse_test_case_yaml(MINIMAL).unwrap(), &action, 15)
            .unwrap_err();

    assert!(matches!(
        error,
        TestingError::ActionNotComposite { ref using }
            if using == "node20"
    ));
}

#[test]
fn rejects_secret_looking_keys_before_remote_mutation() {
    let yaml = MINIMAL.replace(
        "environment: {}",
        "environment:\n  DEPLOY_TOKEN: value",
    );
    let action = parse_action_definition(
        "action.yml",
        "name: Smoke\nruns:\n  using: composite\n  steps: []\n",
    )
    .unwrap();

    let error =
        normalize_test_case(parse_test_case_yaml(&yaml).unwrap(), &action, 15)
            .unwrap_err();

    assert!(matches!(
        error,
        TestingError::SecretLikeKey { ref key }
            if key == "DEPLOY_TOKEN"
    ));
}

#[test]
fn rejects_unknown_fields_and_non_ubuntu_runners() {
    assert!(parse_test_case_yaml(&MINIMAL.replace(
        "inputs: {}",
        "inputz: {}"
    ))
    .is_err());

    let action = parse_action_definition(
        "action.yml",
        "name: Smoke\nruns:\n  using: composite\n  steps: []\n",
    )
    .unwrap();
    let windows = MINIMAL.replace("ubuntu-latest", "windows-latest");

    assert!(normalize_test_case(
        parse_test_case_yaml(&windows).unwrap(),
        &action,
        15,
    )
    .is_err());
}
```

- [ ] **Step 2: Run the tests and verify the red state**

Run: `cargo test -p workbench-domain --test test_plan`

Expected: FAIL because test-case and plan types do not exist.

- [ ] **Step 3: Implement the strict schema and normalization**

```rust
// Add to crates/workbench-domain/src/testing/error.rs
#[error("action runtime `{using}` is not composite")]
ActionNotComposite { using: String },

#[error("test data key `{key}` looks secret-bearing")]
SecretLikeKey { key: String },
```

```rust
// crates/workbench-domain/src/testing/case.rs
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
```

```rust
// crates/workbench-domain/src/testing/plan.rs
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
```

Export the new modules from `testing/mod.rs`.

- [ ] **Step 4: Run domain tests**

Run: `cargo test -p workbench-domain --test test_plan && cargo test -p workbench-domain`

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/workbench-domain/src/testing crates/workbench-domain/tests/test_plan.rs
git commit -m "feat(domain): normalize remote action tests"
```

### Task 3: Generate Deterministic Remote-Test Workflows

**Files:**
- Create: `crates/workbench-domain/src/testing/workflow.rs`
- Modify: `crates/workbench-domain/src/testing/mod.rs`
- Test: `crates/workbench-domain/tests/workflow_generation.rs`
- Create: `crates/workbench-domain/tests/snapshots/workflow_generation__minimal_remote_test.snap`

**Interfaces:**
- Consumes: normalized `TestPlan`, session id, and policy branch prefix.
- Produces:
  - `remote_test_branch(prefix: &str, session_id: &str) -> Result<String, TestingError>`
  - `workflow_file_path(session_id: &str) -> Result<String, TestingError>`
  - `generate_workflow(plan: &TestPlan, session_id: &str, branch_name: &str) -> Result<String, TestingError>`
  - `RESULT_ARTIFACT_NAME`
  - `RESULT_MANIFEST_FILE`.

- [ ] **Step 1: Write the failing golden test**

```rust
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
```

- [ ] **Step 2: Run the golden test and verify the red state**

Run: `cargo test -p workbench-domain --test workflow_generation`

Expected: FAIL because workflow helpers do not exist.

- [ ] **Step 3: Implement deterministic generation**

```rust
// crates/workbench-domain/src/testing/workflow.rs
use std::collections::BTreeMap;

use serde::Serialize;

use super::{TestPlan, TestingError};

pub const RESULT_ARTIFACT_NAME: &str = "github-workbench-result";
pub const RESULT_MANIFEST_FILE: &str = "github-workbench-result.json";

pub fn remote_test_branch(
    prefix: &str,
    session_id: &str,
) -> Result<String, TestingError> {
    validate_identifier(prefix, "branch prefix")?;
    validate_identifier(session_id, "session id")?;
    Ok(format!("{prefix}/{session_id}"))
}

pub fn workflow_file_path(
    session_id: &str,
) -> Result<String, TestingError> {
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
            run: Some(
                "test \"${{ steps.action-under-test.outcome }}\" = success"
                    .into(),
            ),
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

    serde_yaml::to_string(&document).map_err(|error| {
        TestingError::WorkflowGeneration {
            detail: error.to_string(),
        }
    })
}

fn validate_identifier(
    value: &str,
    description: &str,
) -> Result<(), TestingError> {
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
```

Export the constants and functions from `testing/mod.rs`.

- [ ] **Step 4: Approve the deterministic snapshot and rerun**

Run:

```bash
INSTA_UPDATE=always cargo test -p workbench-domain --test workflow_generation
cargo test -p workbench-domain --test workflow_generation
```

Expected: both commands PASS and the snapshot contains one `ubuntu-latest` job, fixed `contents: read`, a branch-filtered push trigger, unique concurrency, manifest creation, artifact upload, and outcome propagation.

- [ ] **Step 5: Commit**

```bash
git add crates/workbench-domain/src/testing crates/workbench-domain/tests/workflow_generation.rs crates/workbench-domain/tests/snapshots
git commit -m "feat(domain): generate remote test workflows"
```

### Task 4: Evaluate Result Manifests and Log Assertions

**Files:**
- Modify: `crates/workbench-domain/Cargo.toml`
- Create: `crates/workbench-domain/src/testing/assertions.rs`
- Modify: `crates/workbench-domain/src/testing/mod.rs`
- Test: `crates/workbench-domain/tests/assertion_evaluation.rs`

**Interfaces:**
- Consumes: `TestAssertions`, terminal GitHub conclusion, optional manifest JSON, downloaded logs, and run URL.
- Produces:
  - `ResultManifest`
  - `AssertionFailure`
  - `AssertionReport`
  - `evaluate_assertions(...) -> AssertionReport`.

- [ ] **Step 1: Write failing assertion tests**

```rust
use workbench_domain::testing::{
    evaluate_assertions, TestAssertions,
};

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
```

- [ ] **Step 2: Run the test and verify the red state**

Run: `cargo test -p workbench-domain --test assertion_evaluation`

Expected: FAIL because the assertion API does not exist.

- [ ] **Step 3: Add serde_json and implement evaluation**

Add `serde_json = { workspace = true }` to `workbench-domain`.

```rust
// crates/workbench-domain/src/testing/assertions.rs
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
                remediation: format!(
                    "Open {run_url} and inspect the artifact-upload step."
                ),
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
                remediation: format!(
                    "Inspect the action and manifest steps at {run_url}."
                ),
            });
        }
    }

    for needle in &assertions.log_contains {
        if !logs.contains(needle) {
            failures.push(AssertionFailure {
                rule: "logs.contains".into(),
                expected: needle.clone(),
                actual: "substring absent".into(),
                remediation: format!(
                    "Inspect downloaded logs or open {run_url}."
                ),
            });
        }
    }

    for needle in &assertions.log_not_contains {
        if logs.contains(needle) {
            failures.push(AssertionFailure {
                rule: "logs.not-contains".into(),
                expected: format!("absence of {needle}"),
                actual: "substring present".into(),
                remediation: format!(
                    "Inspect downloaded logs or open {run_url}."
                ),
            });
        }
    }

    AssertionReport {
        passed: failures.is_empty(),
        manifest,
        failures,
    }
}
```

Export the assertion model from `testing/mod.rs`.

- [ ] **Step 4: Run domain verification**

Run: `cargo test -p workbench-domain --test assertion_evaluation && cargo test -p workbench-domain`

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/workbench-domain/Cargo.toml crates/workbench-domain/src/testing crates/workbench-domain/tests/assertion_evaluation.rs
git commit -m "feat(domain): evaluate remote test evidence"
```

### Task 5: Add Remote-Testing Policy Defaults and Validation

**Files:**
- Modify: `crates/workbench-domain/src/policy/schema.rs`
- Modify: `crates/workbench-domain/src/policy/preset.rs`
- Modify: `crates/workbench-domain/src/policy/load.rs`
- Modify: `crates/workbench-domain/src/policy/mod.rs`
- Test: `crates/workbench-domain/tests/remote_testing_policy.rs`

**Interfaces:**
- Consumes: existing strict `PolicyConfig` parsing.
- Produces:
  - `PolicyConfig::remote_testing`
  - `RemoteTestingConfig`
  - `RetentionHours`
  - defaults `ephemeral-branch`, `github-workbench/test`, `6`, `15`, `0h`, and `72h`.

- [ ] **Step 1: Write failing policy tests**

```rust
use workbench_domain::policy::{
    github_flow_defaults, parse_policy_yaml, RetentionHours,
};

#[test]
fn phase_three_defaults_are_safe() {
    let policy = github_flow_defaults();

    assert_eq!(
        policy.remote_testing.isolation,
        "ephemeral-branch"
    );
    assert_eq!(
        policy.remote_testing.branch_prefix,
        "github-workbench/test"
    );
    assert_eq!(policy.remote_testing.max_matrix_jobs, 6);
    assert_eq!(policy.remote_testing.default_timeout_minutes, 15);
    assert_eq!(
        policy.remote_testing.successful_ref_retention,
        RetentionHours(0)
    );
    assert_eq!(
        policy.remote_testing.failed_ref_retention,
        RetentionHours(72)
    );
}

#[test]
fn parses_exact_remote_testing_keys() {
    let policy = parse_policy_yaml(
        r#"
schema-version: 1
strategy:
  preset: github-flow
  default-branch: main
remote-testing:
  isolation: ephemeral-branch
  branch-prefix: workbench/check
  max-matrix-jobs: 1
  default-timeout-minutes: 20
  successful-ref-retention: 1h
  failed-ref-retention: 96h
"#,
    )
    .unwrap();

    assert_eq!(policy.remote_testing.branch_prefix, "workbench/check");
    assert_eq!(policy.remote_testing.max_matrix_jobs, 1);
    assert_eq!(policy.remote_testing.default_timeout_minutes, 20);
    assert_eq!(
        policy.remote_testing.failed_ref_retention,
        RetentionHours(96)
    );
}

#[test]
fn rejects_unsafe_branch_prefixes_and_zero_timeout() {
    for yaml in [
        "branch-prefix: ../main\ndefault-timeout-minutes: 15",
        "branch-prefix: github-workbench/test\ndefault-timeout-minutes: 0",
    ] {
        let document = format!(
            "schema-version: 1\nstrategy:\n  preset: github-flow\n  default-branch: main\nremote-testing:\n  {}",
            yaml.replace('\n', "\n  ")
        );
        assert!(parse_policy_yaml(&document).is_err());
    }
}
```

- [ ] **Step 2: Run the policy test and verify the red state**

Run: `cargo test -p workbench-domain --test remote_testing_policy`

Expected: FAIL because `remote_testing` and `RetentionHours` do not exist.

- [ ] **Step 3: Implement policy fields and validation**

```rust
// Add to crates/workbench-domain/src/policy/schema.rs
use serde::{de, Deserializer, Serializer};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RetentionHours(pub u64);

impl Serialize for RetentionHours {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&format!("{}h", self.0))
    }
}

impl<'de> Deserialize<'de> for RetentionHours {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        let hours = value
            .strip_suffix('h')
            .ok_or_else(|| de::Error::custom("retention must end in h"))?
            .parse::<u64>()
            .map_err(de::Error::custom)?;
        Ok(Self(hours))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct RemoteTestingConfig {
    #[serde(default = "default_remote_isolation")]
    pub isolation: String,
    #[serde(default = "default_remote_branch_prefix")]
    pub branch_prefix: String,
    #[serde(default = "default_max_matrix_jobs")]
    pub max_matrix_jobs: u16,
    #[serde(default = "default_remote_timeout")]
    pub default_timeout_minutes: u16,
    #[serde(default = "default_successful_retention")]
    pub successful_ref_retention: RetentionHours,
    #[serde(default = "default_failed_retention")]
    pub failed_ref_retention: RetentionHours,
}

impl Default for RemoteTestingConfig {
    fn default() -> Self {
        Self {
            isolation: default_remote_isolation(),
            branch_prefix: default_remote_branch_prefix(),
            max_matrix_jobs: default_max_matrix_jobs(),
            default_timeout_minutes: default_remote_timeout(),
            successful_ref_retention: default_successful_retention(),
            failed_ref_retention: default_failed_retention(),
        }
    }
}

fn default_remote_isolation() -> String {
    "ephemeral-branch".into()
}

fn default_remote_branch_prefix() -> String {
    "github-workbench/test".into()
}

fn default_max_matrix_jobs() -> u16 {
    6
}

fn default_remote_timeout() -> u16 {
    15
}

fn default_successful_retention() -> RetentionHours {
    RetentionHours(0)
}

fn default_failed_retention() -> RetentionHours {
    RetentionHours(72)
}
```

Add this field to `PolicyConfig`:

```rust
#[serde(default)]
pub remote_testing: RemoteTestingConfig,
```

Initialize it in `github_flow_defaults()`:

```rust
remote_testing: RemoteTestingConfig::default(),
```

Validate it after the existing preset validation in `parse_policy_yaml`:

```rust
let remote = &config.remote_testing;
if remote.isolation != "ephemeral-branch" {
    return Err(invalid_policy(
        "policy.remote-testing.isolation",
        "ephemeral-branch",
        remote.isolation.clone(),
        "Phase 3 supports only ephemeral-branch isolation.",
        "Set remote-testing.isolation to ephemeral-branch.",
    ));
}
if remote.max_matrix_jobs == 0 {
    return Err(invalid_policy(
        "policy.remote-testing.max-matrix-jobs",
        "a positive integer",
        "0",
        "Remote test matrix limit must be positive.",
        "Set remote-testing.max-matrix-jobs to at least 1.",
    ));
}
if remote.default_timeout_minutes == 0 {
    return Err(invalid_policy(
        "policy.remote-testing.default-timeout-minutes",
        "a positive integer",
        "0",
        "Remote test timeout must be positive.",
        "Set remote-testing.default-timeout-minutes to at least 1.",
    ));
}

let unsafe_prefix = remote.branch_prefix.is_empty()
    || remote.branch_prefix.starts_with('/')
    || remote.branch_prefix.ends_with('/')
    || remote.branch_prefix.contains("..")
    || remote.branch_prefix.contains('\\')
    || remote
        .branch_prefix
        .chars()
        .any(|character| character.is_control() || character.is_whitespace());

if unsafe_prefix {
    return Err(invalid_policy(
        "policy.remote-testing.branch-prefix",
        "a safe relative Git ref prefix",
        remote.branch_prefix.clone(),
        "Remote test branch prefix is unsafe.",
        "Use a prefix such as github-workbench/test.",
    ));
}
```

Export `RemoteTestingConfig` and `RetentionHours` from `policy/mod.rs`.

- [ ] **Step 4: Run policy and domain tests**

Run: `cargo test -p workbench-domain --test remote_testing_policy && cargo test -p workbench-domain`

Expected: PASS, including existing policy round-trip tests.

- [ ] **Step 5: Commit**

```bash
git add crates/workbench-domain/src/policy crates/workbench-domain/tests/remote_testing_policy.rs
git commit -m "feat(policy): configure remote action testing"
```

### Task 6: Extend Application Ports, Errors, Plans, and Fakes

**Files:**
- Modify: `crates/workbench-domain/src/operations/plan.rs`
- Test: `crates/workbench-domain/tests/operation_plan_remote_test.rs`
- Create: `crates/workbench-application/src/action_tests.rs`
- Modify: `crates/workbench-application/src/error.rs`
- Modify: `crates/workbench-application/src/ports.rs`
- Modify: `crates/workbench-application/src/clock.rs`
- Modify: `crates/workbench-application/src/fakes.rs`
- Modify: `crates/workbench-application/src/lib.rs`

**Interfaces:**
- Consumes: Phase 2 operation, process, storage, clock, id, and policy ports.
- Produces:
  - `GitCommand::CommitPaths`
  - `GitCommand::DeleteRemoteRef`
  - extended `GitClient`
  - exact `GithubClient` contract
  - `TestSessionStore`
  - shared session/result/cleanup records
  - `Sleeper`
  - exit codes 4 and 5.

- [ ] **Step 1: Write failing contract tests**

```rust
// crates/workbench-domain/tests/operation_plan_remote_test.rs
use workbench_domain::operations::plan::GitCommand;

#[test]
fn remote_test_commands_have_stable_step_kinds() {
    assert_eq!(
        GitCommand::CommitPaths {
            message: "chore: add test workflow".into(),
            paths: vec![".github/workflows/test.yml".into()],
        }
        .step_kind(),
        "commit-paths"
    );
    assert_eq!(
        GitCommand::DeleteRemoteRef {
            remote: "origin".into(),
            ref_name: "github-workbench/test/01JABC".into(),
        }
        .step_kind(),
        "delete-remote-ref"
    );
}
```

```rust
// Add to crates/workbench-application/src/error.rs tests
#[test]
fn phase_three_exit_codes_are_stable() {
    assert_eq!(
        AppError::AuthRequired {
            detail: "run gh auth login".into()
        }
        .exit_code(),
        4
    );
    assert_eq!(
        AppError::RemotePending {
            session_id: "01JABC".into()
        }
        .exit_code(),
        5
    );
    assert_eq!(
        AppError::TestCaseInvalid {
            path: "test.yml".into(),
            detail: "bad runner".into(),
        }
        .exit_code(),
        2
    );
}
```

- [ ] **Step 2: Run tests and verify the red state**

Run:

```bash
cargo test -p workbench-domain --test operation_plan_remote_test
cargo test -p workbench-application error::tests::phase_three_exit_codes_are_stable
```

Expected: FAIL because the variants are absent.

- [ ] **Step 3: Add operation commands**

```rust
// Add variants to GitCommand
CommitPaths {
    message: String,
    paths: Vec<String>,
},
DeleteRemoteRef {
    remote: String,
    ref_name: String,
},
```

Extend `step_kind()`:

```rust
GitCommand::CommitPaths { .. } => "commit-paths",
GitCommand::DeleteRemoteRef { .. } => "delete-remote-ref",
```

- [ ] **Step 4: Add application models**

```rust
// crates/workbench-application/src/action_tests.rs
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use workbench_domain::operations::plan::OperationPlan;
use workbench_domain::policy::RetentionHours;
use workbench_domain::testing::{
    AssertionReport, TestAssertions, TestPlan,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CleanupIdentity {
    pub remote: String,
    pub ref_name: String,
    pub session_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExpectedRemoteRef {
    pub identity: CleanupIdentity,
    pub commit_sha: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemoteTestSessionPlan {
    pub project_id: String,
    pub repo_root: PathBuf,
    pub owner: String,
    pub repo: String,
    pub remote: String,
    pub base_sha: String,
    pub session_id: String,
    pub workflow_file_name: String,
    pub workflow_path: String,
    pub workflow_yaml: String,
    pub test_plan: TestPlan,
    pub assertions: TestAssertions,
    pub successful_ref_retention: RetentionHours,
    pub failed_ref_retention: RetentionHours,
    pub cleanup_identity: CleanupIdentity,
    pub git_plan: OperationPlan,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemoteTestResult {
    pub session_id: String,
    pub run_id: u64,
    pub run_url: String,
    pub conclusion: String,
    pub passed: bool,
    pub assertion_report: AssertionReport,
    pub manifest_path: Option<PathBuf>,
    pub logs_path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoredSessionState {
    pub plan: RemoteTestSessionPlan,
    pub pushed_sha: Option<String>,
    pub result: Option<RemoteTestResult>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TestSessionStatus {
    Planned,
    Pushed,
    Queued,
    InProgress,
    Passed,
    Failed,
}
```

- [ ] **Step 5: Add GitHub and session ports**

Add `use serde::{Deserialize, Serialize};` to `ports.rs`, then add:

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkflowRunSummary {
    pub id: u64,
    pub head_sha: String,
    pub path: String,
    pub status: String,
    pub conclusion: Option<String>,
    pub html_url: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkflowRunDetail {
    pub id: u64,
    pub head_sha: String,
    pub path: String,
    pub status: String,
    pub conclusion: Option<String>,
    pub html_url: String,
}

pub trait GithubClient {
    fn auth_status(&self) -> Result<(), AppError>;

    fn list_workflow_runs(
        &self,
        owner: &str,
        repo: &str,
        head_sha: &str,
        workflow_file_name: &str,
    ) -> Result<Vec<WorkflowRunSummary>, AppError>;

    fn get_workflow_run(
        &self,
        owner: &str,
        repo: &str,
        run_id: u64,
    ) -> Result<WorkflowRunDetail, AppError>;

    fn download_artifact_zip(
        &self,
        owner: &str,
        repo: &str,
        run_id: u64,
        artifact_name: &str,
        dest_dir: &Path,
    ) -> Result<PathBuf, AppError>;

    fn download_run_logs(
        &self,
        owner: &str,
        repo: &str,
        run_id: u64,
        dest_path: &Path,
    ) -> Result<PathBuf, AppError>;
}

pub trait Sleeper {
    fn sleep(&self, duration: std::time::Duration);
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestSessionRecord {
    pub id: String,
    pub project_id: String,
    pub session_id: String,
    pub commit_sha: String,
    pub remote_ref: String,
    pub workflow_name: String,
    pub run_id: Option<u64>,
    pub status: TestSessionStatus,
    pub result_json: String,
    pub evidence_dir: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

pub struct NewTestSession<'a> {
    pub id: &'a str,
    pub project_id: &'a str,
    pub session_id: &'a str,
    pub commit_sha: &'a str,
    pub remote_ref: &'a str,
    pub workflow_name: &'a str,
    pub status: TestSessionStatus,
    pub result_json: &'a str,
    pub now: &'a str,
}

pub struct TestSessionUpdate<'a> {
    pub project_id: &'a str,
    pub session_id: &'a str,
    pub run_id: Option<u64>,
    pub status: TestSessionStatus,
    pub result_json: &'a str,
    pub evidence_dir: Option<&'a str>,
    pub now: &'a str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CleanupItemRecord {
    pub id: String,
    pub project_id: String,
    pub resource_kind: String,
    pub resource_id: String,
    pub expected_identity: String,
    pub due_at: String,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
}

pub struct NewCleanupItem<'a> {
    pub id: &'a str,
    pub project_id: &'a str,
    pub resource_kind: &'a str,
    pub resource_id: &'a str,
    pub expected_identity: &'a str,
    pub due_at: &'a str,
    pub now: &'a str,
}

pub trait TestSessionStore {
    fn create_test_session(
        &self,
        session: NewTestSession<'_>,
    ) -> Result<TestSessionRecord, AppError>;

    fn update_test_session(
        &self,
        update: TestSessionUpdate<'_>,
    ) -> Result<(), AppError>;

    fn get_test_session(
        &self,
        project_id: &str,
        session_id: &str,
    ) -> Result<Option<TestSessionRecord>, AppError>;

    fn list_test_sessions(
        &self,
        project_id: &str,
        limit: u32,
    ) -> Result<Vec<TestSessionRecord>, AppError>;

    fn enqueue_cleanup(
        &self,
        item: NewCleanupItem<'_>,
    ) -> Result<CleanupItemRecord, AppError>;

    fn get_cleanup_item(
        &self,
        project_id: &str,
        item_id: &str,
    ) -> Result<Option<CleanupItemRecord>, AppError>;

    fn list_cleanup_items(
        &self,
        project_id: &str,
    ) -> Result<Vec<CleanupItemRecord>, AppError>;

    fn complete_cleanup_item(
        &self,
        item_id: &str,
        now: &str,
    ) -> Result<(), AppError>;
}
```

Extend `GitClient` exactly:

```rust
fn commit_paths(
    &self,
    repo_root: &Path,
    message: &str,
    paths: &[String],
) -> Result<CommandOutput, AppError>;

fn delete_remote_ref(
    &self,
    repo_root: &Path,
    remote: &str,
    ref_name: &str,
) -> Result<CommandOutput, AppError>;

fn rev_parse(
    &self,
    repo_root: &Path,
    reference: &str,
) -> Result<Option<String>, AppError>;
```

- [ ] **Step 6: Add Phase 3 errors**

```rust
// Add to AppError
#[error("GitHub authentication is required: {detail}")]
AuthRequired { detail: String },

#[error("remote test session `{session_id}` is still pending")]
RemotePending { session_id: String },

#[error("gh command failed: {program} {args_summary} (exit {status}): {stderr_redacted}")]
GithubFailed {
    program: String,
    args_summary: String,
    status: i32,
    stderr_redacted: String,
},

#[error("artifact `{artifact_name}` was not found for workflow run `{run_id}`")]
ArtifactNotFound {
    run_id: u64,
    artifact_name: String,
},

#[error("action `{path}` uses unsupported runtime `{using}`")]
ActionNotComposite { path: String, using: String },

#[error("invalid test case `{path}`: {detail}")]
TestCaseInvalid { path: String, detail: String },

#[error("no workflow run matched session `{session_id}` at `{head_sha}`")]
RunNotCorrelated {
    session_id: String,
    head_sha: String,
},

#[error("cleanup ref `{ref_name}` moved from `{expected}` to `{actual}`")]
CleanupRefMoved {
    ref_name: String,
    expected: String,
    actual: String,
},

#[error("cleanup item `{item_id}` has invalid identity: {detail}")]
CleanupIdentityMismatch {
    item_id: String,
    detail: String,
},

#[error("remote test assertions failed for `{session_id}`: {failures:?}")]
AssertionFailed {
    session_id: String,
    failures: Vec<String>,
},
```

Map errors:

```rust
AppError::AuthRequired { .. } => 4,
AppError::RemotePending { .. } => 5,
AppError::ActionNotComposite { .. }
| AppError::TestCaseInvalid { .. }
| AppError::Usage { .. }
| AppError::Io { .. } => 2,
```

Add user-report remediation for authentication, resumable pending/correlation failures, moved refs, and assertion failures.

- [ ] **Step 7: Extend deterministic fakes**

Add:
- `FakeGit.refs: RefCell<BTreeMap<String, String>>`.
- recording implementations for `commit_paths`, `delete_remote_ref`, and `rev_parse`.
- `FakeGithub` with queued run-list/run-detail responses, auth error injection, call recording, and fixture artifact/log writers.
- `FakeStore.sessions` and `FakeStore.cleanup`.
- `FakeSleeper` recording requested durations.
- implementations of every `TestSessionStore` method using mutex-protected vectors.

Ensure `FakeGit::commit_paths` records `GitCommand::CommitPaths`, updates `HEAD` to a deterministic SHA, and `FakeGit::delete_remote_ref` records `DeleteRemoteRef`. Update every existing `FakeGit` initializer in Phase 2 tests to initialize the new `refs` field so the prior test suite continues compiling.

- [ ] **Step 8: Run contract verification**

Run:

```bash
cargo test -p workbench-domain --test operation_plan_remote_test
cargo test -p workbench-application
cargo check -p workbench-application
```

Expected: PASS.

- [ ] **Step 9: Commit**

```bash
git add crates/workbench-domain/src/operations/plan.rs crates/workbench-domain/tests/operation_plan_remote_test.rs crates/workbench-application/src
git commit -m "feat(application): define remote test ports and models"
```

### Task 7: Persist Test Sessions and Cleanup Items in SQLite

**Files:**
- Modify: `crates/workbench-storage/src/migrations.rs`
- Create: `crates/workbench-storage/src/migrations/002_remote_tests.sql`
- Modify: `crates/workbench-storage/src/sqlite.rs`
- Test: `crates/workbench-storage/tests/remote_test_store.rs`

**Interfaces:**
- Consumes: `TestSessionStore` records from Task 6.
- Produces: migration 002 and complete `SqliteStore: TestSessionStore`.

- [ ] **Step 1: Write failing storage round-trip tests**

```rust
use tempfile::tempdir;
use workbench_application::action_tests::TestSessionStatus;
use workbench_application::ports::{
    NewCleanupItem, NewProject, NewTestSession, OperationStore,
    TestSessionStore,
};
use workbench_storage::SqliteStore;

#[test]
fn migration_two_round_trips_sessions_and_cleanup() {
    let temp = tempdir().unwrap();
    let store = SqliteStore::open(&temp.path().join("workbench.db")).unwrap();

    store
        .upsert_project(NewProject {
            id: "project-1",
            local_path: "/repo",
            github_host: Some("github.com"),
            owner: Some("acme"),
            repo: Some("widgets"),
            remote_name: Some("origin"),
            now: "2026-08-24T00:00:00Z",
        })
        .unwrap();

    store
        .create_test_session(NewTestSession {
            id: "row-1",
            project_id: "project-1",
            session_id: "01JABC",
            commit_sha: "abc123",
            remote_ref: "github-workbench/test/01JABC",
            workflow_name: "github-workbench-test-01JABC.yml",
            status: TestSessionStatus::Pushed,
            result_json: r#"{"plan":{},"pushed_sha":"abc123","result":null}"#,
            now: "2026-08-24T00:00:00Z",
        })
        .unwrap();

    let session = store
        .get_test_session("project-1", "01JABC")
        .unwrap()
        .unwrap();
    assert_eq!(session.commit_sha, "abc123");
    assert_eq!(session.status, TestSessionStatus::Pushed);

    store
        .enqueue_cleanup(NewCleanupItem {
            id: "cleanup-1",
            project_id: "project-1",
            resource_kind: "remote-git-ref",
            resource_id: "origin/github-workbench/test/01JABC",
            expected_identity: r#"{"commit_sha":"abc123"}"#,
            due_at: "2026-08-24T00:00:00Z",
            now: "2026-08-24T00:00:00Z",
        })
        .unwrap();

    assert_eq!(
        store.list_cleanup_items("project-1").unwrap().len(),
        1
    );
}

#[test]
fn migrations_are_idempotent() {
    let temp = tempdir().unwrap();
    let path = temp.path().join("workbench.db");

    SqliteStore::open(&path).unwrap();
    SqliteStore::open(&path).unwrap();
}
```

- [ ] **Step 2: Run storage tests and verify the red state**

Run: `cargo test -p workbench-storage --test remote_test_store`

Expected: FAIL because migration 002 and `TestSessionStore` are absent.

- [ ] **Step 3: Add migration 002**

```sql
-- crates/workbench-storage/src/migrations/002_remote_tests.sql
CREATE TABLE test_sessions (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES projects(id),
    session_key TEXT NOT NULL,
    commit_sha TEXT NOT NULL,
    remote_ref TEXT NOT NULL,
    workflow_name TEXT NOT NULL,
    run_id INTEGER,
    status TEXT NOT NULL,
    result_json TEXT NOT NULL,
    evidence_dir TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    UNIQUE(project_id, session_key)
);

CREATE TABLE cleanup_items (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES projects(id),
    resource_kind TEXT NOT NULL,
    resource_id TEXT NOT NULL,
    expected_identity TEXT NOT NULL,
    due_at TEXT NOT NULL,
    status TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE INDEX idx_test_sessions_project_updated
    ON test_sessions(project_id, updated_at DESC);

CREATE INDEX idx_cleanup_items_project_status_due
    ON cleanup_items(project_id, status, due_at);
```

Register it:

```rust
const MIGRATIONS: &[(&str, &str)] = &[
    ("001_initial", include_str!("migrations/001_initial.sql")),
    (
        "002_remote_tests",
        include_str!("migrations/002_remote_tests.sql"),
    ),
];
```

- [ ] **Step 4: Implement all session-store SQL operations**

Use these exact statements:

```sql
INSERT INTO test_sessions
(id, project_id, session_key, commit_sha, remote_ref, workflow_name,
 run_id, status, result_json, evidence_dir, created_at, updated_at)
VALUES (?1, ?2, ?3, ?4, ?5, ?6, NULL, ?7, ?8, NULL, ?9, ?9)
```

```sql
UPDATE test_sessions
SET run_id = ?1, status = ?2, result_json = ?3,
    evidence_dir = ?4, updated_at = ?5
WHERE project_id = ?6 AND session_key = ?7
```

```sql
SELECT id, project_id, session_key, commit_sha, remote_ref,
       workflow_name, run_id, status, result_json, evidence_dir,
       created_at, updated_at
FROM test_sessions
WHERE project_id = ?1 AND session_key = ?2
```

```sql
SELECT id, project_id, session_key, commit_sha, remote_ref,
       workflow_name, run_id, status, result_json, evidence_dir,
       created_at, updated_at
FROM test_sessions
WHERE project_id = ?1
ORDER BY updated_at DESC
LIMIT ?2
```

```sql
INSERT INTO cleanup_items
(id, project_id, resource_kind, resource_id, expected_identity,
 due_at, status, created_at, updated_at)
VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'pending', ?7, ?7)
```

```sql
UPDATE cleanup_items
SET status = 'completed', updated_at = ?1
WHERE id = ?2 AND status = 'pending'
```

Serialize `TestSessionStatus` with `serde_json`; convert `run_id` through `i64::try_from`/`u64::try_from` and return `AppError::Storage` on overflow.

- [ ] **Step 5: Run storage verification**

Run:

```bash
cargo test -p workbench-storage --test remote_test_store
cargo test -p workbench-storage
```

Expected: PASS, including existing operation-journal tests.

- [ ] **Step 6: Commit**

```bash
git add crates/workbench-storage/src crates/workbench-storage/tests/remote_test_store.rs
git commit -m "feat(storage): persist remote test sessions"
```

### Task 8: Implement the Process-Based gh Adapter

**Files:**
- Modify: `Cargo.toml`
- Modify: `crates/workbench-github/Cargo.toml`
- Create: `crates/workbench-github/src/client.rs`
- Create: `crates/workbench-github/src/env.rs`
- Create: `crates/workbench-github/src/parser.rs`
- Modify: `crates/workbench-github/src/lib.rs`
- Create: `crates/workbench-github/tests/process_github_client.rs`
- Create: `crates/workbench-github/tests/fixtures/workflow_runs.json`
- Create: `crates/workbench-github/tests/fixtures/workflow_run_completed.json`
- Create: `crates/workbench-github/tests/fixtures/run_logs.txt`

**Interfaces:**
- Consumes: `GithubClient`, `ProcessRunner`, `CommandSpec`, and `AppError`.
- Produces: `ProcessGithubClient<R>` using only `gh` argv.

- [ ] **Step 1: Add dependencies**

Add to workspace dependencies:

```toml
workbench-github = { path = "crates/workbench-github" }
```

Set the adapter manifest:

```toml
[dependencies]
workbench-application = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }

[dev-dependencies]
tempfile = { workspace = true }
pretty_assertions = { workspace = true }
```

- [ ] **Step 2: Add recorded fixtures**

```json
{
  "workflow_runs": [
    {
      "id": 42,
      "head_sha": "abc123",
      "path": ".github/workflows/github-workbench-test-01JABC.yml",
      "status": "queued",
      "conclusion": null,
      "html_url": "https://github.com/acme/widgets/actions/runs/42"
    },
    {
      "id": 41,
      "head_sha": "different",
      "path": ".github/workflows/other.yml",
      "status": "completed",
      "conclusion": "success",
      "html_url": "https://github.com/acme/widgets/actions/runs/41"
    }
  ]
}
```

```json
{
  "id": 42,
  "head_sha": "abc123",
  "path": ".github/workflows/github-workbench-test-01JABC.yml",
  "status": "completed",
  "conclusion": "success",
  "html_url": "https://github.com/acme/widgets/actions/runs/42"
}
```

`run_logs.txt`:

```text
Set up job
Run action under test
Upload completed
Upload result manifest
```

- [ ] **Step 3: Write failing adapter tests**

```rust
use std::cell::RefCell;
use std::collections::VecDeque;
use std::path::PathBuf;

use workbench_application::ports::{
    CommandOutput, CommandSpec, GithubClient, ProcessRunner,
};
use workbench_application::AppError;
use workbench_github::ProcessGithubClient;

struct RecordingRunner {
    calls: RefCell<Vec<CommandSpec>>,
    outputs: RefCell<VecDeque<CommandOutput>>,
}

impl ProcessRunner for RecordingRunner {
    fn run(&self, spec: &CommandSpec) -> Result<CommandOutput, AppError> {
        self.calls.borrow_mut().push(spec.clone());
        Ok(self.outputs.borrow_mut().pop_front().unwrap())
    }
}

#[test]
fn lists_and_filters_runs_with_argv_only() {
    let runner = RecordingRunner {
        calls: RefCell::new(Vec::new()),
        outputs: RefCell::new(VecDeque::from([CommandOutput {
            exit_code: 0,
            stdout: include_str!("fixtures/workflow_runs.json").into(),
            stderr: String::new(),
        }])),
    };
    let client =
        ProcessGithubClient::with_program(runner, PathBuf::from("/repo"), "gh-fixture");

    let runs = client
        .list_workflow_runs(
            "acme",
            "widgets",
            "abc123",
            "github-workbench-test-01JABC.yml",
        )
        .unwrap();

    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0].id, 42);

    let call = &client.runner().calls.borrow()[0];
    assert_eq!(call.program, "gh-fixture");
    assert_eq!(
        call.args,
        vec![
            "api",
            "--method",
            "GET",
            "repos/acme/widgets/actions/runs",
            "-f",
            "head_sha=abc123",
            "-f",
            "per_page=100"
        ]
    );
}

#[test]
fn auth_failure_maps_to_exit_four_error() {
    let runner = RecordingRunner {
        calls: RefCell::new(Vec::new()),
        outputs: RefCell::new(VecDeque::from([CommandOutput {
            exit_code: 1,
            stdout: String::new(),
            stderr: "not logged in".into(),
        }])),
    };
    let client =
        ProcessGithubClient::with_program(runner, PathBuf::from("/repo"), "gh");

    assert!(matches!(
        client.auth_status().unwrap_err(),
        AppError::AuthRequired { .. }
    ));
}

#[test]
fn insufficient_api_scope_maps_to_exit_four_error() {
    let runner = RecordingRunner {
        calls: RefCell::new(Vec::new()),
        outputs: RefCell::new(VecDeque::from([CommandOutput {
            exit_code: 1,
            stdout: String::new(),
            stderr: "HTTP 403: Resource not accessible by personal access token"
                .into(),
        }])),
    };
    let client =
        ProcessGithubClient::with_program(runner, PathBuf::from("/repo"), "gh");

    assert!(matches!(
        client
            .get_workflow_run("acme", "widgets", 42)
            .unwrap_err(),
        AppError::AuthRequired { .. }
    ));
}
```

- [ ] **Step 4: Run the tests and verify the red state**

Run: `cargo test -p workbench-github`

Expected: FAIL because `ProcessGithubClient` does not exist.

- [ ] **Step 5: Implement parsing and the process client**

```rust
// crates/workbench-github/src/parser.rs
use serde::Deserialize;
use workbench_application::ports::{
    WorkflowRunDetail, WorkflowRunSummary,
};
use workbench_application::AppError;

#[derive(Deserialize)]
struct RunsResponse {
    workflow_runs: Vec<WorkflowRunSummary>,
}

pub fn parse_runs(
    json: &str,
    head_sha: &str,
    workflow_file_name: &str,
) -> Result<Vec<WorkflowRunSummary>, AppError> {
    let response: RunsResponse =
        serde_json::from_str(json).map_err(parse_error)?;
    Ok(response
        .workflow_runs
        .into_iter()
        .filter(|run| {
            run.head_sha == head_sha
                && run
                    .path
                    .rsplit('/')
                    .next()
                    .is_some_and(|name| name == workflow_file_name)
        })
        .collect())
}

pub fn parse_run(json: &str) -> Result<WorkflowRunDetail, AppError> {
    serde_json::from_str(json).map_err(parse_error)
}

fn parse_error(error: serde_json::Error) -> AppError {
    AppError::GithubFailed {
        program: "gh".into(),
        args_summary: "parse JSON response".into(),
        status: 0,
        stderr_redacted: error.to_string(),
    }
}
```

```rust
// crates/workbench-github/src/env.rs
pub fn sanitized_env() -> Vec<(String, String)> {
    [
        "PATH",
        "HOME",
        "USERPROFILE",
        "XDG_CONFIG_HOME",
        "GH_HOST",
        "GH_TOKEN",
        "GITHUB_TOKEN",
        "NO_COLOR",
    ]
    .into_iter()
    .filter_map(|key| {
        std::env::var(key)
            .ok()
            .map(|value| (key.to_string(), value))
    })
    .collect()
}
```

```rust
// crates/workbench-github/src/client.rs
use std::path::{Path, PathBuf};

use workbench_application::ports::{
    CommandOutput, CommandSpec, GithubClient, ProcessRunner,
    WorkflowRunDetail, WorkflowRunSummary,
};
use workbench_application::redact::{bound_output, redact};
use workbench_application::AppError;

use crate::env::sanitized_env;
use crate::parser::{parse_run, parse_runs};

pub struct ProcessGithubClient<R> {
    runner: R,
    cwd: PathBuf,
    gh_program: String,
}

impl<R> ProcessGithubClient<R> {
    pub fn new(runner: R, cwd: PathBuf) -> Self {
        Self {
            runner,
            cwd,
            gh_program: std::env::var("GWW_GH_PROGRAM")
                .unwrap_or_else(|_| "gh".into()),
        }
    }

    pub fn with_program(
        runner: R,
        cwd: PathBuf,
        program: impl Into<String>,
    ) -> Self {
        Self {
            runner,
            cwd,
            gh_program: program.into(),
        }
    }

    pub fn runner(&self) -> &R {
        &self.runner
    }
}

impl<R: ProcessRunner> ProcessGithubClient<R> {
    fn run(&self, args: Vec<String>) -> Result<CommandOutput, AppError> {
        self.runner
            .run(&CommandSpec {
                program: self.gh_program.clone(),
                args: args.clone(),
                cwd: self.cwd.clone(),
                env: sanitized_env(),
            })
            .map_err(|error| match error {
                AppError::GitUnavailable { detail } => AppError::GithubFailed {
                    program: self.gh_program.clone(),
                    args_summary: args.join(" "),
                    status: -1,
                    stderr_redacted: bound_output(&redact(&detail)),
                },
                other => other,
            })
    }

    fn checked(&self, args: Vec<String>) -> Result<CommandOutput, AppError> {
        let output = self.run(args.clone())?;
        if output.exit_code == 0 {
            Ok(output)
        } else {
            let stderr_redacted = bound_output(&redact(&output.stderr));
            if is_auth_failure(&output.stderr) {
                return Err(AppError::AuthRequired {
                    detail: stderr_redacted,
                });
            }
            Err(AppError::GithubFailed {
                program: self.gh_program.clone(),
                args_summary: args.join(" "),
                status: output.exit_code,
                stderr_redacted,
            })
        }
    }
}

fn is_auth_failure(stderr: &str) -> bool {
    let message = stderr.to_ascii_lowercase();
    [
        "http 401",
        "http 403",
        "not logged in",
        "authentication",
        "insufficient scope",
        "resource not accessible by personal access token",
    ]
    .iter()
    .any(|needle| message.contains(needle))
}

impl<R: ProcessRunner> GithubClient for ProcessGithubClient<R> {
    fn auth_status(&self) -> Result<(), AppError> {
        let output = self.run(vec!["auth".into(), "status".into()])?;
        if output.exit_code == 0 {
            Ok(())
        } else {
            Err(AppError::AuthRequired {
                detail: bound_output(&redact(&output.stderr)),
            })
        }
    }

    fn list_workflow_runs(
        &self,
        owner: &str,
        repo: &str,
        head_sha: &str,
        workflow_file_name: &str,
    ) -> Result<Vec<WorkflowRunSummary>, AppError> {
        let output = self.checked(vec![
            "api".into(),
            "--method".into(),
            "GET".into(),
            format!("repos/{owner}/{repo}/actions/runs"),
            "-f".into(),
            format!("head_sha={head_sha}"),
            "-f".into(),
            "per_page=100".into(),
        ])?;
        parse_runs(&output.stdout, head_sha, workflow_file_name)
    }

    fn get_workflow_run(
        &self,
        owner: &str,
        repo: &str,
        run_id: u64,
    ) -> Result<WorkflowRunDetail, AppError> {
        let output = self.checked(vec![
            "api".into(),
            format!("repos/{owner}/{repo}/actions/runs/{run_id}"),
        ])?;
        parse_run(&output.stdout)
    }

    fn download_artifact_zip(
        &self,
        owner: &str,
        repo: &str,
        run_id: u64,
        artifact_name: &str,
        dest_dir: &Path,
    ) -> Result<PathBuf, AppError> {
        std::fs::create_dir_all(dest_dir).map_err(|error| AppError::Io {
            path: dest_dir.display().to_string(),
            detail: error.to_string(),
        })?;
        let download = self.checked(vec![
            "run".into(),
            "download".into(),
            run_id.to_string(),
            "--repo".into(),
            format!("{owner}/{repo}"),
            "--name".into(),
            artifact_name.into(),
            "--dir".into(),
            dest_dir.display().to_string(),
        ]);
        if let Err(error) = download {
            let missing = matches!(
                &error,
                AppError::GithubFailed {
                    stderr_redacted,
                    ..
                } if {
                    let message = stderr_redacted.to_ascii_lowercase();
                    message.contains("no artifacts found")
                        || message.contains("no valid artifacts found")
                }
            );
            if missing {
                return Err(AppError::ArtifactNotFound {
                    run_id,
                    artifact_name: artifact_name.into(),
                });
            }
            return Err(error);
        }
        Ok(dest_dir.to_path_buf())
    }

    fn download_run_logs(
        &self,
        owner: &str,
        repo: &str,
        run_id: u64,
        dest_path: &Path,
    ) -> Result<PathBuf, AppError> {
        let output = self.checked(vec![
            "run".into(),
            "view".into(),
            run_id.to_string(),
            "--repo".into(),
            format!("{owner}/{repo}"),
            "--log".into(),
        ])?;
        std::fs::write(dest_path, output.stdout).map_err(|error| {
            AppError::Io {
                path: dest_path.display().to_string(),
                detail: error.to_string(),
            }
        })?;
        Ok(dest_path.to_path_buf())
    }
}
```

Export `ProcessGithubClient` from `lib.rs`.

- [ ] **Step 6: Complete adapter tests**

Add tests for:
- completed run parsing;
- nonzero `gh api` mapping to redacted `GithubFailed`;
- HTTP 401/403 and insufficient-scope failures mapping to `AuthRequired`;
- artifact download argv and returned directory;
- absent artifacts mapping to `ArtifactNotFound`;
- log download writing `run_logs.txt`;
- no test invoking a real `gh` executable.

Run: `cargo test -p workbench-github`

Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add Cargo.toml crates/workbench-github
git commit -m "feat(github): add gh process adapter"
```

### Task 9: Implement Commit, Rev-Parse, and Exact Remote-Ref Deletion

**Files:**
- Modify: `crates/workbench-git/src/argv.rs`
- Modify: `crates/workbench-git/src/client.rs`
- Modify: `crates/workbench-git/src/lib.rs`
- Modify: `crates/workbench-application/src/executor.rs`
- Modify: `crates/workbench-application/src/fakes.rs`
- Modify: `crates/workbench-git/tests/git_integration.rs`

**Interfaces:**
- Consumes: extended `GitCommand` and `GitClient`.
- Produces:
  - generated-path-only commits;
  - optional commit resolution;
  - non-force exact remote deletion;
  - executor journaling for both new commands.

- [ ] **Step 1: Write failing argv tests**

```rust
use workbench_domain::operations::plan::GitCommand;
use workbench_git::command_argvs;

#[test]
fn commit_paths_stages_and_commits_only_named_paths() {
    let commands = command_argvs(&GitCommand::CommitPaths {
        message: "chore: add remote action test".into(),
        paths: vec![
            ".github/workflows/github-workbench-test-01JABC.yml".into(),
        ],
    });

    assert_eq!(
        commands,
        vec![
            vec![
                "add",
                "--",
                ".github/workflows/github-workbench-test-01JABC.yml",
            ],
            vec![
                "commit",
                "-m",
                "chore: add remote action test",
                "--",
                ".github/workflows/github-workbench-test-01JABC.yml",
            ],
        ]
    );
}

#[test]
fn delete_ref_uses_a_non_force_empty_source_refspec() {
    let commands = command_argvs(&GitCommand::DeleteRemoteRef {
        remote: "github".into(),
        ref_name: "github-workbench/test/01JABC".into(),
    });

    assert_eq!(
        commands,
        vec![vec![
            "push",
            "--",
            "github",
            ":refs/heads/github-workbench/test/01JABC",
        ]]
    );
    assert!(!commands
        .iter()
        .flatten()
        .any(|argument| argument.contains("force")));
}
```

- [ ] **Step 2: Run focused tests and verify the red state**

Run: `cargo test -p workbench-git argv`

Expected: FAIL because `command_argvs` is absent.

- [ ] **Step 3: Replace the single-argv renderer**

```rust
pub fn command_argvs(cmd: &GitCommand) -> Vec<Vec<String>> {
    let commands = match cmd {
        GitCommand::Fetch { remote } => {
            vec![vec!["fetch".into(), "--".into(), remote.clone()]]
        }
        GitCommand::CreateBranch { name, start_point } => vec![vec![
            "checkout".into(),
            "-b".into(),
            name.clone(),
            start_point.clone(),
            "--".into(),
        ]],
        GitCommand::PushRef {
            remote,
            local_ref,
            remote_ref,
            set_upstream,
        } => {
            let mut args = vec!["push".into()];
            if *set_upstream {
                args.push("-u".into());
            }
            args.extend([
                "--".into(),
                remote.clone(),
                format!("{local_ref}:{remote_ref}"),
            ]);
            vec![args]
        }
        GitCommand::CommitPaths { message, paths } => {
            let mut add = vec!["add".into(), "--".into()];
            add.extend(paths.iter().cloned());
            let mut commit =
                vec!["commit".into(), "-m".into(), message.clone(), "--".into()];
            commit.extend(paths.iter().cloned());
            vec![add, commit]
        }
        GitCommand::DeleteRemoteRef { remote, ref_name } => vec![vec![
            "push".into(),
            "--".into(),
            remote.clone(),
            format!(":refs/heads/{ref_name}"),
        ]],
    };

    for args in &commands {
        assert_no_force(args);
    }
    commands
}
```

Update `describe_command` to render each argv line prefixed by `git`, joined with `\n`. Export `command_argvs` from `crates/workbench-git/src/lib.rs`, remove the old `command_argv` export, and update all Phase 2 call sites and tests to use the new nested argv shape.

- [ ] **Step 4: Implement GitClient methods**

```rust
fn commit_paths(
    &self,
    repo_root: &Path,
    message: &str,
    paths: &[String],
) -> Result<CommandOutput, AppError> {
    if paths.is_empty() {
        return Err(AppError::Usage {
            message: "commit_paths requires at least one path".into(),
        });
    }

    let command = GitCommand::CommitPaths {
        message: message.into(),
        paths: paths.to_vec(),
    };
    let mut combined = CommandOutput {
        exit_code: 0,
        stdout: String::new(),
        stderr: String::new(),
    };
    for args in command_argvs(&command) {
        let output = self.run_checked(repo_root, args)?;
        combined.stdout.push_str(&output.stdout);
        combined.stderr.push_str(&output.stderr);
    }
    Ok(combined)
}

fn delete_remote_ref(
    &self,
    repo_root: &Path,
    remote: &str,
    ref_name: &str,
) -> Result<CommandOutput, AppError> {
    let command = GitCommand::DeleteRemoteRef {
        remote: remote.into(),
        ref_name: ref_name.into(),
    };
    self.run_checked(
        repo_root,
        command_argvs(&command).into_iter().next().unwrap(),
    )
}

fn rev_parse(
    &self,
    repo_root: &Path,
    reference: &str,
) -> Result<Option<String>, AppError> {
    let output = self.run(
        repo_root,
        vec![
            "rev-parse".into(),
            "--verify".into(),
            "--quiet".into(),
            "--end-of-options".into(),
            format!("{reference}^{{commit}}"),
        ],
    )?;
    if output.exit_code == 0 {
        Ok(nonempty_trimmed(&output.stdout))
    } else {
        Ok(None)
    }
}
```

Update existing fetch/create/push methods to select their single argv from `command_argvs`.

- [ ] **Step 5: Extend the executor allowlist**

Add executor branches:

```rust
GitCommand::CommitPaths { message, paths } => {
    git.commit_paths(root, message, paths)
}
GitCommand::DeleteRemoteRef { remote, ref_name } => {
    git.delete_remote_ref(root, remote, ref_name)
}
```

Treat `CommitPaths`, `DeleteRemoteRef`, `CreateBranch`, and `PushRef` as mutating steps. Add descriptions:

```rust
GitCommand::CommitPaths { paths, .. } => {
    format!("Committed generated paths: {}.", paths.join(", "))
}
GitCommand::DeleteRemoteRef { remote, ref_name } => {
    format!("Deleted temporary ref `{remote}/{ref_name}`.")
}
```

- [ ] **Step 6: Add a real-Git integration test**

Extend the existing temporary bare-remote harness to:
1. create `.github/workflows/github-workbench-test-01JABC.yml`;
2. call `commit_paths`;
3. assert `git show --name-only --format=` lists only that path;
4. push the temporary branch;
5. fetch the remote;
6. assert `rev_parse("refs/remotes/<remote>/github-workbench/test/01JABC")` equals the pushed SHA;
7. call `delete_remote_ref`;
8. assert `git ls-remote --heads` no longer reports the branch.

- [ ] **Step 7: Run Git and application verification**

Run:

```bash
cargo test -p workbench-git
cargo test -p workbench-application
```

Expected: PASS and no generated argv contains a force option.

- [ ] **Step 8: Commit**

```bash
git add crates/workbench-git crates/workbench-application/src/executor.rs crates/workbench-application/src/fakes.rs
git commit -m "feat(git): support remote test commits and cleanup"
```

### Task 10: Implement Discovery, Planning, Execution, Resume, and Cleanup Use Cases

**Files:**
- Modify: `crates/workbench-application/Cargo.toml`
- Create: `crates/workbench-application/src/use_cases/action_discovery.rs`
- Create: `crates/workbench-application/src/use_cases/remote_test.rs`
- Create: `crates/workbench-application/src/use_cases/test_sessions.rs`
- Create: `crates/workbench-application/src/use_cases/cleanup.rs`
- Modify: `crates/workbench-application/src/use_cases/mod.rs`
- Modify: `crates/workbench-application/src/lib.rs`
- Create: `crates/workbench-application/tests/support/mod.rs`
- Test: `crates/workbench-application/tests/remote_test_plan.rs`
- Test: `crates/workbench-application/tests/remote_test_execute.rs`
- Test: `crates/workbench-application/tests/remote_test_resume.rs`
- Test: `crates/workbench-application/tests/cleanup.rs`

**Interfaces:**
- Consumes: all Phase 3 domain APIs, application ports, Phase 2 project mapping, operation executor, clock, ids, and policy.
- Produces:
  - `discover_action_tests`
  - `plan_remote_test`
  - `execute_remote_test`
  - `watch_session`
  - `list_sessions`
  - `get_session_result`
  - `list_cleanup`
  - `plan_cleanup`
  - `execute_cleanup`.

- [ ] **Step 1: Write failing planning tests**

```rust
mod support;

use support::RemoteTestHarness;
use workbench_application::use_cases::remote_test::plan_remote_test;
use workbench_application::AppError;
use workbench_domain::operations::plan::GitCommand;

#[test]
fn clean_repository_plans_generated_only_ephemeral_push() {
    let harness = RemoteTestHarness::new();
    let plan = plan_remote_test(
        &harness.git,
        &harness.store,
        &harness.policy,
        &harness.ids,
        harness.repo.path(),
        "smoke-composite",
        None,
    )
    .unwrap();

    assert_eq!(
        plan.workflow_path,
        format!(
            ".github/workflows/github-workbench-test-{}.yml",
            plan.session_id
        )
    );
    assert_eq!(
        plan.cleanup_identity.ref_name,
        format!("github-workbench/test/{}", plan.session_id)
    );
    assert!(matches!(
        plan.git_plan.commands.as_slice(),
        [
            GitCommand::CreateBranch { .. },
            GitCommand::CommitPaths { paths, .. },
            GitCommand::PushRef {
                set_upstream: false,
                ..
            }
        ] if paths == &[plan.workflow_path.clone()]
    ));
    assert!(plan.workflow_yaml.contains("ubuntu-latest"));
}

#[test]
fn dirty_repository_is_rejected_before_files_or_store_change() {
    let harness = RemoteTestHarness::new();
    harness.git.snapshot.borrow_mut().dirty_paths =
        vec!["src/lib.rs".into()];

    let error = plan_remote_test(
        &harness.git,
        &harness.store,
        &harness.policy,
        &harness.ids,
        harness.repo.path(),
        "smoke-composite",
        None,
    )
    .unwrap_err();

    assert!(matches!(error, AppError::DirtyWorkingTree { .. }));
    assert!(harness.store.sessions.lock().unwrap().is_empty());
}
```

- [ ] **Step 2: Write failing execution, resume, and cleanup tests**

```rust
mod support;

use support::RemoteTestHarness;
use workbench_application::ports::TestSessionStore;
use workbench_application::use_cases::cleanup::execute_cleanup;
use workbench_application::use_cases::remote_test::{
    execute_remote_test, watch_session,
};
use workbench_application::AppError;
use workbench_domain::operations::plan::GitCommand;

#[test]
fn execution_authenticates_pushes_downloads_and_persists_result() {
    let harness = RemoteTestHarness::completed_success();
    let plan = harness.plan();

    let result = execute_remote_test(
        &harness.git,
        &harness.github,
        &harness.store,
        &harness.clock,
        &harness.ids,
        &harness.sleeper,
        &plan,
        harness.evidence.path(),
    )
    .unwrap();

    assert!(result.passed);
    assert_eq!(result.run_id, 42);
    assert!(harness.github.calls().first().unwrap().starts_with("auth"));
    assert_eq!(
        harness.git.executed.borrow().iter().filter(|command| {
            matches!(command, GitCommand::PushRef { .. })
        }).count(),
        1
    );
    assert_eq!(
        harness.store.list_cleanup_items(&plan.project_id).unwrap().len(),
        1
    );
}

#[test]
fn watch_resumes_a_stored_push_without_repush() {
    let harness = RemoteTestHarness::stored_pending_then_success();

    let result = watch_session(
        &harness.git,
        &harness.github,
        &harness.store,
        &harness.clock,
        &harness.ids,
        &harness.sleeper,
        harness.repo.path(),
        "01JABC",
        harness.evidence.path(),
        true,
    )
    .unwrap();

    assert!(result.passed);
    assert!(harness.git.executed.borrow().iter().all(|command| {
        !matches!(
            command,
            GitCommand::CreateBranch { .. }
                | GitCommand::CommitPaths { .. }
                | GitCommand::PushRef { .. }
        )
    }));
}

#[test]
fn cleanup_ref_move_is_refused_without_delete() {
    let harness = RemoteTestHarness::cleanup_with_remote_sha("moved-sha");

    let error = execute_cleanup(
        &harness.git,
        &harness.github,
        &harness.store,
        &harness.clock,
        &harness.ids,
        harness.repo.path(),
        "cleanup-1",
    )
    .unwrap_err();

    assert!(matches!(error, AppError::CleanupRefMoved { .. }));
    assert!(harness.git.executed.borrow().iter().all(|command| {
        !matches!(command, GitCommand::DeleteRemoteRef { .. })
    }));
}
```

In `tests/support/mod.rs`, implement `RemoteTestHarness` from public fakes with these constructors:

```rust
pub struct RemoteTestHarness {
    pub repo: tempfile::TempDir,
    pub evidence: tempfile::TempDir,
    pub git: FakeGit,
    pub github: FakeGithub,
    pub store: FakeStore,
    pub policy: FakePolicy,
    pub clock: FakeClock,
    pub ids: FakeIds,
    pub sleeper: FakeSleeper,
}

impl RemoteTestHarness {
    pub fn new() -> Self;
    pub fn completed_success() -> Self;
    pub fn stored_pending_then_success() -> Self;
    pub fn cleanup_with_remote_sha(actual_sha: &str) -> Self;
    pub fn plan(&self) -> RemoteTestSessionPlan;
}
```

`new()` must write the approved minimal `action.yml` and test YAML, seed one mapped `ProjectRecord`, and configure a clean snapshot at `abc123`. `completed_success()` queues one matching run and one completed successful run plus the exact manifest/log fixture. `stored_pending_then_success()` inserts a `Pushed` session whose `StoredSessionState.pushed_sha` is `abc123`. `cleanup_with_remote_sha()` inserts an `ExpectedRemoteRef` for `abc123` and seeds the fake remote-tracking ref with the supplied actual SHA.

- [ ] **Step 3: Run application tests and verify the red state**

Run:

```bash
cargo test -p workbench-application --test remote_test_plan
cargo test -p workbench-application --test remote_test_execute
cargo test -p workbench-application --test remote_test_resume
cargo test -p workbench-application --test cleanup
```

Expected: FAIL because the use cases do not exist.

- [ ] **Step 4: Implement action and test discovery**

Define:

```rust
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

pub fn discover_action_tests(
    repo_root: &Path,
) -> Result<ActionTestCatalog, AppError>;
```

Implement `discover_action_tests(repo_root)` with recursive `read_dir`, skipping `.git`, `target`, `node_modules`, `dist`, and `build`. Recognize only `action.yml`, `action.yaml`, and `.github-workbench/tests/*.yml`. Parse unsupported actions into catalog entries with warnings rather than aborting the whole discovery.

- [ ] **Step 5: Implement remote-test planning**

Use this public signature:

```rust
pub fn plan_remote_test<G, S, P, I>(
    git: &G,
    store: &S,
    policy_source: &P,
    ids: &I,
    path: &Path,
    test_name: &str,
    remote_flag: Option<&str>,
) -> Result<RemoteTestSessionPlan, AppError>
where
    G: GitClient,
    S: OperationStore,
    P: PolicySource,
    I: IdGenerator;
```

The implementation must:
1. resolve root and snapshot;
2. reject dirty or detached state;
3. load policy;
4. resolve the selected/mapped remote;
5. require an existing mapped project with GitHub owner/repository identity;
6. locate the named test and its referenced action;
7. map domain test errors to `ActionNotComposite` or `TestCaseInvalid`;
8. generate one ULID session id;
9. normalize with policy timeout;
10. generate branch, workflow path, and workflow YAML;
11. copy policy successful/failed retention into `RemoteTestSessionPlan`;
12. scan existing `.github/workflows` text for `push:` and add a warning rationale when found;
13. create this exact command sequence:

```rust
vec![
    GitCommand::CreateBranch {
        name: branch.clone(),
        start_point: base_sha.clone(),
    },
    GitCommand::CommitPaths {
        message: format!(
            "chore: add GitHub Workbench test {}",
            session_id
        ),
        paths: vec![workflow_path.clone()],
    },
    GitCommand::PushRef {
        remote: remote.clone(),
        local_ref: branch.clone(),
        remote_ref: branch.clone(),
        set_upstream: false,
    },
]
```

The resulting `OperationPlan` uses:
- kind `remote-action-test`;
- risk `RiskClass::Medium`;
- preconditions naming the clean-tree and unchanged-HEAD requirements;
- rationale naming the generated file, temporary ref, artifact, and manual cleanup.

Planning must not write files, create refs, call `gh`, or write SQLite.

- [ ] **Step 6: Implement execution and persisted state**

Use:

```rust
pub fn execute_remote_test<G, H, S, C, I, L>(
    git: &G,
    github: &H,
    store: &S,
    clock: &C,
    ids: &I,
    sleeper: &L,
    plan: &RemoteTestSessionPlan,
    evidence_root: &Path,
) -> Result<RemoteTestResult, AppError>
where
    G: GitClient,
    H: GithubClient,
    S: OperationStore + TestSessionStore,
    C: Clock,
    I: IdGenerator,
    L: Sleeper;
```

Implement this sequence exactly:

```rust
github.auth_status()?;

let snapshot = git.snapshot(&plan.repo_root)?;
if !snapshot.dirty_paths.is_empty() {
    return Err(AppError::DirtyWorkingTree {
        paths: snapshot.dirty_paths,
    });
}
if snapshot.head_oid.as_deref() != Some(plan.base_sha.as_str()) {
    return Err(AppError::OperationFailed {
        message: "HEAD changed after remote-test planning".into(),
        changed: Vec::new(),
        unchanged: vec![
            "The generated workflow was not written.".into(),
            "The temporary branch was not pushed.".into(),
        ],
        retry_safe: true,
        remediation: "Create a new remote-test plan from the current HEAD."
            .into(),
    });
}

let workflow_path = plan.repo_root.join(&plan.workflow_path);
std::fs::create_dir_all(workflow_path.parent().unwrap())
    .map_err(|error| io_error(&workflow_path, error))?;
std::fs::write(&workflow_path, &plan.workflow_yaml)
    .map_err(|error| io_error(&workflow_path, error))?;

execute_plan(
    git,
    store,
    clock,
    ids,
    &plan.project_id,
    &snapshot,
    &plan.git_plan,
)?;

let pushed_sha = git
    .rev_parse(&plan.repo_root, "HEAD")?
    .ok_or_else(|| AppError::OperationFailed {
        message: "could not resolve the pushed test commit".into(),
        changed: vec![format!(
            "Temporary branch `{}` was created.",
            plan.cleanup_identity.ref_name
        )],
        unchanged: vec!["Run correlation was not started.".into()],
        retry_safe: false,
        remediation: "Inspect the operation journal before retrying.".into(),
    })?;
```

Use this helper:

```rust
fn io_error(path: &Path, error: std::io::Error) -> AppError {
    AppError::Io {
        path: path.display().to_string(),
        detail: error.to_string(),
    }
}
```

Serialize `StoredSessionState`, create the `test_sessions` row with `Pushed`, then call `watch_session` in waiting mode. If monitoring fails after push, leave the stored session intact so the watch command can resume.

- [ ] **Step 7: Implement correlation, polling, download, and assertion**

Use:

```rust
pub fn watch_session<G, H, S, C, I, L>(
    git: &G,
    github: &H,
    store: &S,
    clock: &C,
    ids: &I,
    sleeper: &L,
    path: &Path,
    session_id: &str,
    evidence_root: &Path,
    wait: bool,
) -> Result<RemoteTestResult, AppError>
where
    G: GitClient,
    H: GithubClient,
    S: OperationStore + TestSessionStore,
    C: Clock,
    I: IdGenerator,
    L: Sleeper;
```

Behavior:
- authenticate before correlation or polling;
- load project and stored session;
- deserialize `StoredSessionState`;
- if a stored result exists, return it when passed or reconstruct `AssertionFailed` when failed;
- if `run_id` is absent, call `list_workflow_runs` with stored pushed SHA and workflow filename;
- choose the sole exact match and reject ambiguous matches;
- return `RunNotCorrelated` if no run appears after waiting retries;
- store run id immediately after correlation;
- poll `get_workflow_run`;
- map `queued` and `in_progress` into session status;
- when `wait == false`, persist status and return `RemotePending`;
- when `wait == true`, sleep three seconds and continue;
- after `completed`, require a conclusion;
- download the named artifact into `<evidence_root>/<session_id>`, treating `ArtifactNotFound` as an absent manifest so assertion evaluation produces the required run-URL remediation;
- download logs into `<evidence_root>/<session_id>/run.log`;
- read `<evidence_root>/<session_id>/github-workbench-result.json` when present;
- read the downloaded logs, call `evaluate_assertions` against that evidence, and overwrite `run.log` with `redact(&logs)` before persisting its path;
- persist `RemoteTestResult`;
- enqueue exactly one cleanup item with serialized `ExpectedRemoteRef`;
- compute due time from the retention copied into the stored plan;
- return `AssertionFailed` after persistence when the report failed.

Limit correlation to 40 attempts and terminal polling to 600 attempts; exhaustion returns `RemotePending` so the stored session remains resumable.

Use `time::OffsetDateTime::parse` with `Rfc3339`, add `time::Duration::hours`, and format with `Rfc3339` for cleanup `due_at`.

- [ ] **Step 8: Implement listing and cleanup**

Public APIs:

```rust
pub fn list_sessions<G, S>(
    git: &G,
    store: &S,
    path: &Path,
) -> Result<Vec<TestSessionRecord>, AppError>
where
    G: GitClient,
    S: OperationStore + TestSessionStore;

pub fn get_session_result<G, S>(
    git: &G,
    store: &S,
    path: &Path,
    session_id: &str,
) -> Result<Option<RemoteTestResult>, AppError>
where
    G: GitClient,
    S: OperationStore + TestSessionStore;

pub fn list_cleanup<G, S>(
    git: &G,
    store: &S,
    path: &Path,
) -> Result<Vec<CleanupItemRecord>, AppError>
where
    G: GitClient,
    S: OperationStore + TestSessionStore;

pub fn plan_cleanup<G, S>(
    git: &G,
    store: &S,
    path: &Path,
    item_id: &str,
) -> Result<
    (OperationPlan, RepositorySnapshot, CleanupItemRecord),
    AppError,
>
where
    G: GitClient,
    S: OperationStore + TestSessionStore;

pub fn execute_cleanup<G, H, S, C, I>(
    git: &G,
    github: &H,
    store: &S,
    clock: &C,
    ids: &I,
    path: &Path,
    item_id: &str,
) -> Result<ExecuteOutcome, AppError>
where
    G: GitClient,
    H: GithubClient,
    S: OperationStore + TestSessionStore,
    C: Clock,
    I: IdGenerator;
```

Cleanup planning returns a medium-risk `OperationPlan` containing only:

```rust
GitCommand::DeleteRemoteRef {
    remote: expected.identity.remote.clone(),
    ref_name: expected.identity.ref_name.clone(),
}
```

Cleanup execution must:
1. reject any item whose `resource_kind` is not `remote-git-ref` or whose status is not `pending`;
2. load the session named by `expected.identity.session_id`;
3. require the cleanup identity to equal the stored `RemoteTestSessionPlan.cleanup_identity`, the expected SHA to equal `StoredSessionState.pushed_sha`, and `resource_id` to equal `<remote>/<ref_name>`;
4. return `CleanupIdentityMismatch` before GitHub or Git mutation for any malformed or mismatched identity;
5. call `github.auth_status`;
6. fetch the recorded remote;
7. resolve `refs/remotes/<remote>/<ref_name>`;
8. compare it with `expected.commit_sha`;
9. return `CleanupRefMoved` for any mismatch or missing ref;
10. execute and journal `DeleteRemoteRef`;
11. mark the cleanup item completed.

- [ ] **Step 9: Run all application tests and dependency-boundary check**

Run:

```bash
cargo test -p workbench-application
if cargo tree -p workbench-application | rg 'workbench-(git|storage|github)'; then exit 1; fi
```

Expected:
- all application tests PASS;
- the dependency check prints no adapter crate names;
- resume tests record no branch creation, commit, or push;
- moved-ref cleanup records no delete operation.

- [ ] **Step 10: Commit**

```bash
git add crates/workbench-application
git commit -m "feat(application): orchestrate remote action tests"
```

### Task 11: Wire the gww Action, Runs, and Cleanup Commands

**Files:**
- Modify: `crates/workbench-cli/Cargo.toml`
- Modify: `crates/workbench-cli/src/args.rs`
- Modify: `crates/workbench-cli/src/main.rs`
- Modify: `crates/workbench-cli/src/render.rs`
- Create: `crates/workbench-cli/tests/cli_remote_action.rs`

**Interfaces:**
- Consumes: application use cases plus concrete Git, GitHub, and SQLite adapters.
- Produces:
  - `gww action discover`
  - `gww action test [name] [--yes]`
  - `gww runs list`
  - `gww runs watch <session-id>`
  - `gww cleanup list`
  - `gww cleanup run <item-id> [--yes]`.

- [ ] **Step 1: Add CLI parsing tests**

```rust
#[test]
fn parses_phase_three_commands() {
    assert!(matches!(
        Cli::try_parse_from(["gww", "action", "discover"])
            .unwrap()
            .command,
        Some(Commands::Action {
            command: ActionCommands::Discover
        })
    ));

    assert!(matches!(
        Cli::try_parse_from([
            "gww",
            "action",
            "test",
            "smoke-composite",
            "--yes"
        ])
        .unwrap()
        .command,
        Some(Commands::Action {
            command: ActionCommands::Test {
                name: Some(_),
                yes: true
            }
        })
    ));

    assert!(matches!(
        Cli::try_parse_from(["gww", "runs", "watch", "01JABC"])
            .unwrap()
            .command,
        Some(Commands::Runs {
            command: RunsCommands::Watch { session_id }
        }) if session_id == "01JABC"
    ));

    assert!(matches!(
        Cli::try_parse_from([
            "gww",
            "cleanup",
            "run",
            "cleanup-1",
            "--yes"
        ])
        .unwrap()
        .command,
        Some(Commands::Cleanup {
            command: CleanupCommands::Run {
                item_id,
                yes: true
            }
        }) if item_id == "cleanup-1"
    ));
}
```

- [ ] **Step 2: Run parsing tests and verify the red state**

Run: `cargo test -p workbench-cli args::tests::parses_phase_three_commands`

Expected: FAIL because the command enums do not exist.

- [ ] **Step 3: Add clap command types**

```rust
#[derive(Debug, Subcommand)]
pub enum ActionCommands {
    Discover,
    Test {
        name: Option<String>,
        #[arg(long)]
        yes: bool,
    },
}

#[derive(Debug, Subcommand)]
pub enum RunsCommands {
    List,
    Watch { session_id: String },
}

#[derive(Debug, Subcommand)]
pub enum CleanupCommands {
    List,
    Run {
        item_id: String,
        #[arg(long)]
        yes: bool,
    },
}
```

Add corresponding nested variants to `Commands`.

- [ ] **Step 4: Add concrete dependencies and wiring**

Add:

```toml
workbench-github = { workspace = true }
```

Construct the GitHub adapter from the current repository root:

```rust
let github =
    ProcessGithubClient::new(StdProcessRunner, root.clone());
```

For `action test`:
1. select the provided test name, or require exactly one discovered test;
2. call `plan_remote_test`;
3. render the full remote plan before confirmation;
4. abort without mutation when confirmation is denied;
5. call `execute_remote_test`;
6. print session id, run URL, conclusion, assertion result, manifest path, logs path, and cleanup hint.

For `runs watch`, pass `wait = true` and never invoke planning.

For cleanup, render the exact remote and ref from `plan_cleanup`, confirm, then call `execute_cleanup`.

- [ ] **Step 5: Add renderers**

Render remote plans with:
- risk;
- preconditions;
- generated workflow path;
- generated branch;
- Git argv;
- owner/repository;
- expected runner;
- artifact name;
- estimated single job;
- cleanup policy.

Render session lists as:

```text
SESSION     STATUS       RUN       REMOTE REF
01JABC      passed       42        github-workbench/test/01JABC
```

Render cleanup lists as:

```text
ITEM         STATUS       DUE                    RESOURCE
cleanup-1    pending      2026-08-24T00:00:00Z   origin/github-workbench/test/01JABC
```

- [ ] **Step 6: Add a fixture-driven CLI integration test**

Create a local bare remote and configure Git’s `url.<local-path>.insteadOf` mapping for `git@github.com:acme/widgets.git`. Create:
- one composite `action.yml`;
- one valid test YAML;
- a fixture `gh` executable selected through `GWW_GH_PROGRAM`.

The fixture executable must:
- return success for `auth status`;
- return recorded run JSON for `api`;
- create `github-workbench-result.json` for `run download`;
- return recorded logs for `run view --log`.

Assert:
1. `gww open .` succeeds;
2. `gww action discover` lists the composite action;
3. `gww action test smoke-composite --yes` reports pass and one session id;
4. the bare remote contains the temporary branch;
5. `gww runs list` contains the session;
6. `gww runs watch <session-id>` does not create another commit or push;
7. `gww cleanup list` contains one item;
8. `gww cleanup run <item-id> --yes` removes only that remote ref.

- [ ] **Step 7: Run CLI verification**

Run:

```bash
cargo test -p workbench-cli
cargo run -p workbench-cli -- action --help
cargo run -p workbench-cli -- runs --help
cargo run -p workbench-cli -- cleanup --help
```

Expected: tests PASS and help output lists the Phase 3 subcommands while existing Phase 2 commands remain present.

- [ ] **Step 8: Commit**

```bash
git add crates/workbench-cli
git commit -m "feat(cli): expose remote action tests"
```

### Task 12: Scaffold the Thin Tauri Action Tests Desktop

**Files:**
- Modify: `Cargo.toml`
- Create: `crates/workbench-desktop/package.json`
- Create: `crates/workbench-desktop/package-lock.json`
- Create: `crates/workbench-desktop/index.html`
- Create: `crates/workbench-desktop/tsconfig.json`
- Create: `crates/workbench-desktop/vite.config.ts`
- Create: `crates/workbench-desktop/src/main.tsx`
- Create: `crates/workbench-desktop/src/App.tsx`
- Create: `crates/workbench-desktop/src/api.ts`
- Create: `crates/workbench-desktop/src/styles.css`
- Create: `crates/workbench-desktop/src-tauri/Cargo.toml`
- Create: `crates/workbench-desktop/src-tauri/build.rs`
- Create: `crates/workbench-desktop/src-tauri/tauri.conf.json`
- Create: `crates/workbench-desktop/src-tauri/capabilities/default.json`
- Create: `crates/workbench-desktop/src-tauri/src/main.rs`
- Create: `crates/workbench-desktop/src-tauri/src/lib.rs`
- Create: `crates/workbench-desktop/src-tauri/src/commands.rs`
- Create: `crates/workbench-desktop/src-tauri/tests/action_tests_commands.rs`

**Interfaces:**
- Consumes: the same application use cases as the CLI.
- Produces: Action Tests list/start/watch/result UI; cleanup remains a CLI hint.

- [ ] **Step 1: Add the workspace member and current Tauri/React dependencies**

Add:

```toml
"crates/workbench-desktop/src-tauri",
```

to workspace members.

Create the Rust manifest:

```toml
[package]
name = "workbench-desktop"
version.workspace = true
edition.workspace = true
license.workspace = true
rust-version.workspace = true

[lib]
name = "workbench_desktop"
crate-type = ["lib", "cdylib", "staticlib"]

[[bin]]
name = "workbench-desktop"
path = "src/main.rs"

[build-dependencies]
tauri-build = { version = "2", features = [] }

[dependencies]
tauri = { version = "2", features = [] }
serde = { workspace = true }
serde_json = { workspace = true }
workbench-application = { workspace = true }
workbench-git = { workspace = true }
workbench-github = { workspace = true }
workbench-storage = { workspace = true }

[dev-dependencies]
tempfile = { workspace = true }
```

Create `package.json` using the latest compatible React, Vite, TypeScript, Tauri API, and Tauri CLI releases:

```json
{
  "name": "github-workflow-workbench-desktop",
  "private": true,
  "version": "0.1.0",
  "type": "module",
  "scripts": {
    "dev": "vite",
    "build": "tsc --noEmit && vite build",
    "tauri": "tauri"
  },
  "dependencies": {
    "@tauri-apps/api": "^2.0.0",
    "react": "^19.0.0",
    "react-dom": "^19.0.0"
  },
  "devDependencies": {
    "@tauri-apps/cli": "^2.0.0",
    "@types/react": "^19.0.0",
    "@types/react-dom": "^19.0.0",
    "@vitejs/plugin-react": "^5.0.0",
    "typescript": "^5.0.0",
    "vite": "^7.0.0"
  }
}
```

Run `npm install` in `crates/workbench-desktop` to resolve and commit the lockfile.

- [ ] **Step 2: Write a failing Tauri command contract test**

```rust
use tempfile::tempdir;
use workbench_desktop::commands::list_action_tests_from_root;

#[test]
fn list_command_delegates_to_application_discovery() {
    let repo = tempdir().unwrap();
    std::fs::write(
        repo.path().join("action.yml"),
        "name: Smoke\nruns:\n  using: composite\n  steps: []\n",
    )
    .unwrap();
    std::fs::create_dir_all(
        repo.path().join(".github-workbench/tests"),
    )
    .unwrap();
    std::fs::write(
        repo.path()
            .join(".github-workbench/tests/smoke.yml"),
        "schema-version: 1\nname: smoke\naction:\n  path: .\nrunner:\n  os: [ubuntu-latest]\nexpect:\n  conclusion: success\n",
    )
    .unwrap();

    let catalog = list_action_tests_from_root(repo.path()).unwrap();

    assert_eq!(catalog.actions.len(), 1);
    assert_eq!(catalog.tests.len(), 1);
}
```

- [ ] **Step 3: Run the desktop test and verify the red state**

Run: `cargo test -p workbench-desktop`

Expected: FAIL because the desktop crate and commands are not implemented.

- [ ] **Step 4: Implement Tauri configuration and commands**

Use four Tauri commands:
- `list_action_tests(repo_root)`
- `start_action_test(repo_root, test_name, confirmed)`
- `watch_action_test(repo_root, session_id)`
- `get_action_test_result(repo_root, session_id)`.

`start_action_test` returns the plan when `confirmed == false`; when true it calls the same planner and executor used by the CLI. `watch_action_test` calls `watch_session` with `wait == false` and converts `RemotePending` into a serializable pending response. No command duplicates branch, workflow, assertion, or cleanup rules.

Register commands:

```rust
tauri::Builder::default()
    .manage(DesktopState::new()?)
    .invoke_handler(tauri::generate_handler![
        commands::list_action_tests,
        commands::start_action_test,
        commands::watch_action_test,
        commands::get_action_test_result,
    ])
    .run(tauri::generate_context!())
    .expect("failed to run GitHub Workflow Workbench");
```

Use `tauri::async_runtime::spawn_blocking` for Git, SQLite, and `gh` work.

- [ ] **Step 5: Implement the React API layer**

```typescript
// crates/workbench-desktop/src/api.ts
import { invoke } from "@tauri-apps/api/core";

export type TestEntry = {
  path: string;
  name: string;
};

export type Catalog = {
  actions: Array<{
    definition: {
      manifest_path: string;
      name: string;
    };
    supported: boolean;
    warning: string | null;
  }>;
  tests: TestEntry[];
};

export type SessionPlan = {
  session_id: string;
  workflow_path: string;
  cleanup_identity: {
    remote: string;
    ref_name: string;
    session_id: string;
  };
  git_plan: {
    summary: string;
    preconditions: string[];
  };
};

export type TestResult = {
  session_id: string;
  run_id: number;
  run_url: string;
  conclusion: string;
  passed: boolean;
  manifest_path: string | null;
  logs_path: string;
};

export const listActionTests = (repoRoot: string) =>
  invoke<Catalog>("list_action_tests", { repoRoot });

export const startActionTest = (
  repoRoot: string,
  testName: string,
  confirmed: boolean,
) =>
  invoke<{ plan: SessionPlan; result: TestResult | null }>(
    "start_action_test",
    { repoRoot, testName, confirmed },
  );

export const watchActionTest = (
  repoRoot: string,
  sessionId: string,
) =>
  invoke<{ pending: boolean; result: TestResult | null }>(
    "watch_action_test",
    { repoRoot, sessionId },
  );

export const getActionTestResult = (
  repoRoot: string,
  sessionId: string,
) =>
  invoke<TestResult | null>("get_action_test_result", {
    repoRoot,
    sessionId,
  });
```

- [ ] **Step 6: Implement the minimal Action Tests UI**

`App.tsx` must provide:
- repository-path entry;
- refresh/list actions and tests;
- unsupported-action warning;
- start button;
- plan panel with workflow path, remote ref, preconditions, and confirmation;
- polling status;
- pass/fail, conclusion, run link, manifest path, and logs path;
- copyable `gww cleanup list` / `gww cleanup run <item-id>` guidance.

It must not add Home, Changes, pull-request, or cleanup-queue screens.

- [ ] **Step 7: Build and test the desktop**

Run:

```bash
cargo test -p workbench-desktop
npm ci --prefix crates/workbench-desktop
npm run build --prefix crates/workbench-desktop
```

Expected: Rust tests PASS, TypeScript type-checks, and Vite emits `crates/workbench-desktop/dist`.

- [ ] **Step 8: Commit**

```bash
git add Cargo.toml Cargo.lock crates/workbench-desktop
git commit -m "feat(desktop): add Action Tests Tauri shell"
```

### Task 13: Document Phase 3, Guard CI, and Describe Optional Live E2E

**Files:**
- Modify: `README.md`
- Modify: `docs/architecture.md`
- Create: `docs/superpowers/manual/phase3-live-e2e.md`
- Modify: `.github/workflows/ci.yml`
- Create: `crates/workbench-cli/tests/documentation_contract.rs`

**Interfaces:**
- Consumes: completed CLI, desktop, adapter, and use-case behavior.
- Produces: operator documentation, architecture constraints, live-test procedure, and CI network guard.

- [ ] **Step 1: Write a failing documentation contract test**

```rust
use std::fs;

#[test]
fn phase_three_documentation_names_safety_and_live_boundaries() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..");

    let readme = fs::read_to_string(root.join("README.md")).unwrap();
    let architecture =
        fs::read_to_string(root.join("docs/architecture.md")).unwrap();
    let live = fs::read_to_string(
        root.join("docs/superpowers/manual/phase3-live-e2e.md"),
    )
    .unwrap();

    for command in [
        "gww action discover",
        "gww action test",
        "gww runs watch",
        "gww cleanup run",
    ] {
        assert!(readme.contains(command));
    }

    assert!(architecture.contains(
        "workbench-application does not depend on adapter crates"
    ));
    assert!(architecture.contains("Never force push"));
    assert!(live.contains("disposable repository"));
    assert!(live.contains("GWW_LIVE_E2E=1"));
    assert!(live.contains("not required CI"));
}
```

- [ ] **Step 2: Run the test and verify the red state**

Run: `cargo test -p workbench-cli --test documentation_contract`

Expected: FAIL because the live manual and Phase 3 documentation are absent.

- [ ] **Step 3: Update README**

Document:
- Phase 3 status;
- prerequisite `gh auth login`;
- minimal action/test locations;
- all Phase 3 commands;
- exit codes;
- evidence location under `GWW_DATA_DIR`;
- cleanup identity protection;
- desktop development commands.

Include:

```bash
gww open .
gww action discover
gww action test smoke-composite --yes
gww runs list
gww runs watch <session-id>
gww cleanup list
gww cleanup run <item-id> --yes
```

- [ ] **Step 4: Update architecture documentation**

Record:
- domain/application/adapter/presentation dependency direction;
- `workbench-application does not depend on adapter crates`;
- domain filesystem/process/database prohibition;
- `RemoteTestSessionPlan` versus Git `OperationPlan`;
- persisted resume sequence;
- manifest/log assertion flow;
- `gh` argv-only process execution;
- exact-ref cleanup validation;
- “Never force push” invariant;
- no live GitHub traffic in CI.

- [ ] **Step 5: Write the optional live end-to-end manual**

The manual must require:
1. a disposable repository;
2. authenticated `gh`;
3. a composite action and locked minimal test YAML;
4. `GWW_LIVE_E2E=1`;
5. explicit confirmation that existing push workflows may also run;
6. execution of discover, test, list, watch, and cleanup;
7. verification of manifest/log evidence;
8. verification that moved-ref cleanup refuses deletion;
9. deletion of the disposable repository after completion.

State explicitly that this procedure is “not required CI” and must not run from the default workflow.

- [ ] **Step 6: Harden CI and build the desktop**

Add Linux Tauri prerequisites, Node setup, frontend build, and a deliberately invalid default `gh` program:

```yaml
env:
  GWW_GH_PROGRAM: gww-gh-live-access-is-disabled-in-ci

steps:
  - uses: actions/checkout@v4

  - uses: dtolnay/rust-toolchain@stable
    with:
      components: rustfmt, clippy

  - uses: actions/setup-node@v4
    with:
      node-version: 22
      cache: npm
      cache-dependency-path: crates/workbench-desktop/package-lock.json

  - name: Install Tauri Linux dependencies
    run: |
      sudo apt-get update
      sudo apt-get install -y \
        libwebkit2gtk-4.1-dev \
        libappindicator3-dev \
        librsvg2-dev \
        patchelf

  - name: Install desktop dependencies
    run: npm ci --prefix crates/workbench-desktop

  - name: Build desktop frontend
    run: npm run build --prefix crates/workbench-desktop

  - name: Format
    run: cargo fmt --all -- --check

  - name: Clippy
    run: cargo clippy --workspace --all-targets -- -D warnings

  - name: Test
    run: cargo test --workspace
```

Fixture-driven tests may override `GWW_GH_PROGRAM` with their local executable. No workflow step may authenticate `gh`, pass a token to tests, or run the live manual.

- [ ] **Step 7: Run final verification**

Run:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
npm ci --prefix crates/workbench-desktop
npm run build --prefix crates/workbench-desktop
if cargo tree -p workbench-application | rg 'workbench-(git|storage|github)'; then exit 1; fi
rg -- '--force|--force-with-lease' crates/workbench-git/src
```

Expected:
- formatting succeeds;
- clippy reports no warnings;
- every workspace test passes without live GitHub access;
- desktop frontend builds;
- application dependency output contains no adapter crate;
- the force scan finds only the defensive rejection checks and tests, never generated push argv.

- [ ] **Step 8: Commit**

```bash
git add README.md docs .github/workflows/ci.yml crates/workbench-cli/tests/documentation_contract.rs
git commit -m "docs: complete Phase 3 remote test guidance"
```
