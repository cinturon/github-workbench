# Phase 1 Domain Foundation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Scaffold the GitHub Workflow Workbench Rust workspace and implement pure domain logic for policy v1, GitHub Flow naming, workflow states, and typed operation plans with unit, property, and golden tests.

**Architecture:** A Cargo workspace with `workbench-domain` holding all Phase 1 behavior (no I/O). Adapter crates (`git`, `github`, `storage`) and `workbench-cli` (`gww`) are compile-only stubs. `workbench-application` defines port traits only. Domain modules: `policy`, `repository`, `workflow`, `operations`, `testing` (placeholder).

**Tech Stack:** Rust 2021 edition, `serde` / `serde_yaml`, `thiserror`, `ulid`, `proptest`, `insta`, `pretty_assertions`; GitHub Actions CI on Ubuntu.

## Global Constraints

- License: MIT / Apache-2.0 dual (`LICENSE-MIT`, `LICENSE-APACHE`).
- No Tauri / React in this phase.
- CLI binary name: `gww`.
- Domain crate must not depend on filesystem, Git, GitHub, SQLite, Tokio, reqwest, or Tauri.
- Unknown YAML policy fields are errors.
- GitHub Flow only; branch pattern default `feature/{issue}-{slug}`.
- Spec: `docs/superpowers/specs/2026-08-23-phase1-domain-foundation-design.md`.
- Prefer TDD: failing test → implement → pass → commit per task.
- On Windows PowerShell, use `git commit -m "message"` (no bash heredoc required).

---

## File structure

```text
Cargo.toml
.gitignore
LICENSE-MIT
LICENSE-APACHE
README.md
CONTRIBUTING.md
SECURITY.md
CODE_OF_CONDUCT.md
.github/workflows/ci.yml
docs/architecture.md
docs/product/GITHUB_WORKFLOW_WORKBENCH_DESIGN.md
crates/workbench-domain/Cargo.toml
crates/workbench-domain/src/lib.rs
crates/workbench-domain/src/error.rs
crates/workbench-domain/src/policy/mod.rs
crates/workbench-domain/src/policy/finding.rs
crates/workbench-domain/src/policy/schema.rs
crates/workbench-domain/src/policy/preset.rs
crates/workbench-domain/src/policy/load.rs
crates/workbench-domain/src/repository/mod.rs
crates/workbench-domain/src/workflow/mod.rs
crates/workbench-domain/src/workflow/naming.rs
crates/workbench-domain/src/workflow/state.rs
crates/workbench-domain/src/operations/mod.rs
crates/workbench-domain/src/operations/plan.rs
crates/workbench-domain/src/operations/create_branch.rs
crates/workbench-domain/src/testing/mod.rs
crates/workbench-domain/tests/policy_load.rs
crates/workbench-domain/tests/branch_naming.rs
crates/workbench-domain/tests/workflow_state.rs
crates/workbench-domain/tests/create_branch_plan.rs
crates/workbench-domain/tests/snapshots/…   # created by insta
crates/workbench-application/Cargo.toml
crates/workbench-application/src/lib.rs
crates/workbench-application/src/ports.rs
crates/workbench-application/README.md
crates/workbench-git/Cargo.toml
crates/workbench-git/src/lib.rs
crates/workbench-git/README.md
crates/workbench-github/Cargo.toml
crates/workbench-github/src/lib.rs
crates/workbench-github/README.md
crates/workbench-storage/Cargo.toml
crates/workbench-storage/src/lib.rs
crates/workbench-storage/README.md
crates/workbench-cli/Cargo.toml
crates/workbench-cli/src/main.rs
```

---

### Task 1: Workspace scaffold, licenses, stub crates

**Files:**
- Create: `Cargo.toml`, `.gitignore`, `LICENSE-MIT`, `LICENSE-APACHE`, `README.md`, `CONTRIBUTING.md`, `SECURITY.md`, `CODE_OF_CONDUCT.md`
- Create: all stub crate paths listed above (except domain implementation files beyond empty `lib.rs`)
- Create: `docs/architecture.md`
- Create: `docs/product/GITHUB_WORKFLOW_WORKBENCH_DESIGN.md` (copy from `C:\Users\Jeremy Belt\OneDrive\Documents\S3Uploader\GITHUB_WORKFLOW_WORKBENCH_DESIGN.md`)

**Interfaces:**
- Consumes: none
- Produces: workspace that `cargo check --workspace` compiles; binary `gww` exists

- [ ] **Step 1: Create root `Cargo.toml`**

```toml
[workspace]
resolver = "2"
members = [
    "crates/workbench-domain",
    "crates/workbench-application",
    "crates/workbench-git",
    "crates/workbench-github",
    "crates/workbench-storage",
    "crates/workbench-cli",
]

[workspace.package]
version = "0.1.0"
edition = "2021"
license = "MIT OR Apache-2.0"
repository = "https://github.com/example/github-workbench"
rust-version = "1.78"

[workspace.dependencies]
serde = { version = "1", features = ["derive"] }
serde_yaml = "0.9"
thiserror = "2"
ulid = "1"
proptest = "1"
insta = { version = "1", features = ["yaml"] }
pretty_assertions = "1"
workbench-domain = { path = "crates/workbench-domain" }
workbench-application = { path = "crates/workbench-application" }
```

- [ ] **Step 2: Create `.gitignore`**

```gitignore
/target
**/*.rs.bk
.DS_Store
.idea/
.vscode/
*.swp
/Cargo.lock
```

Note: For a binary workspace, prefer committing `Cargo.lock`. After first successful `cargo generate-lockfile` or `cargo check`, **remove** `/Cargo.lock` from `.gitignore` and commit the lockfile in a later step of this task.

Correct `.gitignore` for this repo:

```gitignore
/target
**/*.rs.bk
.DS_Store
.idea/
.vscode/
*.swp
```

- [ ] **Step 3: Add dual licenses**

Write standard MIT text to `LICENSE-MIT` (copyright `Copyright (c) 2026 GitHub Workflow Workbench contributors`).

Write standard Apache-2.0 text to `LICENSE-APACHE` (from https://www.apache.org/licenses/LICENSE-2.0.txt).

- [ ] **Step 4: Write `README.md`**

```markdown
# GitHub Workflow Workbench

Desktop and CLI assistant that guides GitHub Flow and tests custom GitHub Actions on real runners.

**Status:** Phase 1 — domain foundation (no desktop UI yet).

## License

Licensed under either of

- Apache License, Version 2.0 (`LICENSE-APACHE`)
- MIT license (`LICENSE-MIT`)

at your option.

## Development

```bash
cargo test --workspace
cargo run -p workbench-cli -- --version
```

See `docs/architecture.md` and `docs/superpowers/specs/2026-08-23-phase1-domain-foundation-design.md`.
```

- [ ] **Step 5: Write short `CONTRIBUTING.md`, `SECURITY.md`, `CODE_OF_CONDUCT.md`**

`CONTRIBUTING.md`: point to dual license, `cargo fmt`, `clippy -D warnings`, `cargo test`, and sensitive areas (command execution, credentials) from the product design.

`SECURITY.md`: report vulnerabilities privately; do not file public issues for credential/token bugs.

`CODE_OF_CONDUCT.md`: use Contributor Covenant v2.1 summary or full text with enforcement contact placeholder `maintainers@localhost`.

- [ ] **Step 6: Copy product design and write `docs/architecture.md`**

Copy the design file into `docs/product/GITHUB_WORKFLOW_WORKBENCH_DESIGN.md`.

```markdown
# Architecture

GitHub Workflow Workbench uses a layered Rust core:

1. **Domain** (`workbench-domain`) — pure policy, naming, plans, assertions.
2. **Application** (`workbench-application`) — use cases over ports.
3. **Adapters** — `workbench-git`, `workbench-github`, `workbench-storage`.
4. **Presentation** — `workbench-cli` (`gww`); desktop UI deferred.

Phase 1 implements domain logic only. Adapter and CLI crates are stubs.

Product design: `docs/product/GITHUB_WORKFLOW_WORKBENCH_DESIGN.md`.
Phase 1 spec: `docs/superpowers/specs/2026-08-23-phase1-domain-foundation-design.md`.
```

- [ ] **Step 7: Create stub crates**

`crates/workbench-domain/Cargo.toml`:

```toml
[package]
name = "workbench-domain"
version.workspace = true
edition.workspace = true
license.workspace = true
rust-version.workspace = true
description = "Pure domain logic for GitHub Workflow Workbench"

[dependencies]
serde = { workspace = true }
serde_yaml = { workspace = true }
thiserror = { workspace = true }
ulid = { workspace = true }

[dev-dependencies]
proptest = { workspace = true }
insta = { workspace = true }
pretty_assertions = { workspace = true }
```

`crates/workbench-domain/src/lib.rs`:

```rust
//! Pure domain types and rules for GitHub Workflow Workbench.

pub mod error;
pub mod operations;
pub mod policy;
pub mod repository;
pub mod testing;
pub mod workflow;
```

Create empty module files that compile:

```rust
// error.rs
#![allow(dead_code)]
```

```rust
// policy/mod.rs
#![allow(dead_code)]
```

```rust
// repository/mod.rs
#![allow(dead_code)]
```

```rust
// workflow/mod.rs
#![allow(dead_code)]
```

```rust
// operations/mod.rs
#![allow(dead_code)]
```

```rust
// testing/mod.rs
//! Placeholder for Phase 3 action-test domain types.

#![allow(dead_code)]
```

`crates/workbench-application/Cargo.toml`:

```toml
[package]
name = "workbench-application"
version.workspace = true
edition.workspace = true
license.workspace = true
rust-version.workspace = true

[dependencies]
workbench-domain = { workspace = true }
```

`crates/workbench-application/src/lib.rs`:

```rust
pub mod ports;
```

`crates/workbench-application/src/ports.rs`:

```rust
//! External capability ports. Implementations arrive in later phases.

#![allow(dead_code)]

/// Placeholder until Phase 2 defines Git operations.
pub trait GitClient {}

/// Placeholder until Phase 2/4 defines GitHub operations.
pub trait GitHubClient {}

/// Placeholder until Phase 2 defines operation journaling.
pub trait OperationStore {}
```

`crates/workbench-application/README.md`: `Phase 2+ — application use cases over ports.`

For `workbench-git`, `workbench-github`, `workbench-storage`: each gets `Cargo.toml` with package name matching folder, empty `src/lib.rs` (`//! Stub adapter. Phase 2+.`), and README `Phase 2+ stub.`

`crates/workbench-cli/Cargo.toml`:

```toml
[package]
name = "workbench-cli"
version.workspace = true
edition.workspace = true
license.workspace = true
rust-version.workspace = true

[[bin]]
name = "gww"
path = "src/main.rs"

[dependencies]
workbench-domain = { workspace = true }
workbench-application = { workspace = true }
```

`crates/workbench-cli/src/main.rs`:

```rust
fn main() {
    let mut args = std::env::args().skip(1);
    match args.next().as_deref() {
        Some("--version") | Some("-V") => {
            println!("gww {}", env!("CARGO_PKG_VERSION"));
        }
        Some(other) => {
            eprintln!("gww: command `{other}` is not implemented yet (Phase 1 stub)");
            std::process::exit(2);
        }
        None => {
            eprintln!("gww: usage: gww --version");
            std::process::exit(2);
        }
    }
}
```

- [ ] **Step 8: Verify compile**

Run: `cargo check --workspace`

Expected: success (warnings about empty modules OK; fix errors).

Run: `cargo run -p workbench-cli -- --version`

Expected: prints `gww 0.1.0`

- [ ] **Step 9: Commit**

```bash
git add Cargo.toml Cargo.lock .gitignore LICENSE-MIT LICENSE-APACHE README.md CONTRIBUTING.md SECURITY.md CODE_OF_CONDUCT.md docs crates
git commit -m "chore: scaffold Rust workspace and open-source docs"
```

---

### Task 2: Error types and policy findings

**Files:**
- Modify: `crates/workbench-domain/src/error.rs`
- Create: `crates/workbench-domain/src/policy/finding.rs`
- Modify: `crates/workbench-domain/src/policy/mod.rs`
- Test: unit tests inside `finding.rs` and `error.rs`

**Interfaces:**
- Consumes: none
- Produces:
  - `Severity::{Warning, Blocker}`
  - `PolicyFinding { rule_id, severity, expected, actual, message, remediation }`
  - `WorkbenchError::{InvalidPolicy { findings }, PolicyBlocked { findings }, InvalidBranchName { reason }, ProtectedBranchMisuse { branch }}`

- [ ] **Step 1: Write failing tests in `policy/finding.rs`**

Replace `policy/mod.rs` with:

```rust
pub mod finding;

pub use finding::{PolicyFinding, Severity};
```

Add to `finding.rs` (tests first — types can be incomplete so tests fail to compile/run):

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Warning,
    Blocker,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PolicyFinding {
    pub rule_id: String,
    pub severity: Severity,
    pub expected: String,
    pub actual: String,
    pub message: String,
    pub remediation: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn blocker_finding_serializes_severity_lowercase() {
        let f = PolicyFinding {
            rule_id: "pull-requests.required-base".into(),
            severity: Severity::Blocker,
            expected: "main".into(),
            actual: "develop".into(),
            message: "Feature PRs must target main.".into(),
            remediation: "Change the pull request base to main.".into(),
        };
        let v = serde_yaml::to_value(&f).unwrap();
        assert_eq!(v["severity"].as_str(), Some("blocker"));
    }
}
```

- [ ] **Step 2: Run test**

Run: `cargo test -p workbench-domain blocker_finding_serializes_severity_lowercase -- --nocapture`

Expected: PASS once types exist (write types in Step 1 together if preferred; if splitting, expect compile fail then pass).

- [ ] **Step 3: Implement `error.rs`**

```rust
use crate::policy::PolicyFinding;
use thiserror::Error;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum WorkbenchError {
    #[error("invalid policy: {0} finding(s)", findings.len())]
    InvalidPolicy { findings: Vec<PolicyFinding> },

    #[error("policy blocked operation: {0} finding(s)", findings.len())]
    PolicyBlocked { findings: Vec<PolicyFinding> },

    #[error("invalid branch name: {reason}")]
    InvalidBranchName { reason: String },

    #[error("refusing to use protected branch `{branch}` for feature work")]
    ProtectedBranchMisuse { branch: String },
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::policy::{PolicyFinding, Severity};

    #[test]
    fn display_includes_finding_count() {
        let err = WorkbenchError::InvalidPolicy {
            findings: vec![PolicyFinding {
                rule_id: "schema-version".into(),
                severity: Severity::Blocker,
                expected: "1".into(),
                actual: "2".into(),
                message: "unsupported".into(),
                remediation: "use schema-version 1".into(),
            }],
        };
        assert!(err.to_string().contains("1 finding"));
    }
}
```

Export from `lib.rs`: `pub use error::WorkbenchError;`

- [ ] **Step 4: Run tests**

Run: `cargo test -p workbench-domain`

Expected: PASS for finding + error tests.

- [ ] **Step 5: Commit**

```bash
git add crates/workbench-domain
git commit -m "feat(domain): add policy findings and WorkbenchError"
```

---

### Task 3: Slug normalization and branch naming

**Files:**
- Create: `crates/workbench-domain/src/workflow/naming.rs`
- Modify: `crates/workbench-domain/src/workflow/mod.rs`
- Create: `crates/workbench-domain/tests/branch_naming.rs`

**Interfaces:**
- Consumes: `WorkbenchError`
- Produces:
  - `fn normalize_slug(title: &str) -> Result<String, WorkbenchError>`
  - `fn branch_name(pattern: &str, issue: u64, title: &str) -> Result<String, WorkbenchError>`
  - Pattern tokens: `{issue}`, `{slug}`

- [ ] **Step 1: Write failing integration tests**

`tests/branch_naming.rs`:

```rust
use workbench_domain::workflow::naming::{branch_name, normalize_slug};

#[test]
fn slug_from_issue_title() {
    assert_eq!(
        normalize_slug("Add resumable uploads").unwrap(),
        "add-resumable-uploads"
    );
}

#[test]
fn feature_branch_for_issue_42() {
    let name = branch_name("feature/{issue}-{slug}", 42, "Add resumable uploads").unwrap();
    assert_eq!(name, "feature/42-add-resumable-uploads");
}

#[test]
fn empty_title_is_invalid() {
    assert!(normalize_slug("@@@").is_err());
}

#[test]
fn rejects_double_dot_in_slug_source_after_normalize_path() {
    // normalize strips unsafe sequences; result must not contain ".."
    let slug = normalize_slug("foo..bar").unwrap();
    assert!(!slug.contains(".."));
}
```

- [ ] **Step 2: Run tests — expect FAIL**

Run: `cargo test -p workbench-domain --test branch_naming`

Expected: FAIL (module/function not found) or assert failures.

- [ ] **Step 3: Implement `naming.rs`**

```rust
use crate::error::WorkbenchError;

/// Lowercase, hyphenated slug safe for Git ref path segments.
pub fn normalize_slug(title: &str) -> Result<String, WorkbenchError> {
    let mut out = String::new();
    let mut prev_hyphen = false;
    for ch in title.chars() {
        let c = ch.to_ascii_lowercase();
        if c.is_ascii_alphanumeric() {
            out.push(c);
            prev_hyphen = false;
        } else if !prev_hyphen && !out.is_empty() {
            out.push('-');
            prev_hyphen = true;
        }
    }
    let out = out.trim_matches('-').to_string();
    if out.is_empty() {
        return Err(WorkbenchError::InvalidBranchName {
            reason: "slug is empty after normalization".into(),
        });
    }
    if out.contains("..") || out.starts_with('.') || out.ends_with('.') {
        return Err(WorkbenchError::InvalidBranchName {
            reason: "slug contains prohibited '.' sequences".into(),
        });
    }
    Ok(out)
}

pub fn branch_name(pattern: &str, issue: u64, title: &str) -> Result<String, WorkbenchError> {
    let slug = normalize_slug(title)?;
    let name = pattern
        .replace("{issue}", &issue.to_string())
        .replace("{slug}", &slug);
    validate_branch_ref(&name)?;
    Ok(name)
}

fn validate_branch_ref(name: &str) -> Result<(), WorkbenchError> {
    if name.is_empty()
        || name.contains(' ')
        || name.contains("//")
        || name.contains("..")
        || name.starts_with('/')
        || name.ends_with('/')
        || name.ends_with('.lock')
        || name.contains('@{')
        || name.contains('\\')
        || name.contains('\0')
        || name.chars().any(|c| c.is_ascii_control())
    {
        return Err(WorkbenchError::InvalidBranchName {
            reason: format!("prohibited ref characters in `{name}`"),
        });
    }
    Ok(())
}

#[cfg(test)]
mod property_tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn slug_never_contains_prohibited_ref_chars(title in ".{0,80}") {
            if let Ok(slug) = normalize_slug(&title) {
                prop_assert!(!slug.contains(".."));
                prop_assert!(!slug.contains(' '));
                prop_assert!(!slug.contains('@'));
                prop_assert!(!slug.contains('\\'));
                prop_assert!(!slug.is_empty());
            }
        }
    }
}
```

`workflow/mod.rs`:

```rust
pub mod naming;
pub mod state;
```

Create temporary empty `state.rs` with `#![allow(dead_code)]` if Task 5 not done yet.

- [ ] **Step 4: Run tests — expect PASS**

Run: `cargo test -p workbench-domain --test branch_naming`
Run: `cargo test -p workbench-domain slug_never_contains_prohibited_ref_chars`

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/workbench-domain
git commit -m "feat(domain): add slug normalization and branch naming"
```

---

### Task 4: Policy schema, preset, and load/validate

**Files:**
- Create: `crates/workbench-domain/src/policy/schema.rs`
- Create: `crates/workbench-domain/src/policy/preset.rs`
- Create: `crates/workbench-domain/src/policy/load.rs`
- Modify: `crates/workbench-domain/src/policy/mod.rs`
- Create: `crates/workbench-domain/tests/policy_load.rs`

**Interfaces:**
- Consumes: `PolicyFinding`, `Severity`, `WorkbenchError`
- Produces:
  - `PolicyConfig` (schema-version, strategy, branches, commits, pull_requests, … subset for GitHub Flow)
  - `fn github_flow_defaults() -> PolicyConfig`
  - `fn parse_policy_yaml(yaml: &str) -> Result<PolicyConfig, WorkbenchError>`
  - `fn merge_policy(base: PolicyConfig, overlay: PolicyConfig) -> PolicyConfig` (overlay fields replace)
  - Unknown fields → `InvalidPolicy`

- [ ] **Step 1: Write failing tests**

`tests/policy_load.rs`:

```rust
use workbench_domain::policy::{github_flow_defaults, parse_policy_yaml};
use workbench_domain::WorkbenchError;

#[test]
fn parses_minimal_github_flow_yaml() {
    let yaml = r#"
schema-version: 1
strategy:
  preset: github-flow
  default-branch: main
"#;
    let cfg = parse_policy_yaml(yaml).unwrap();
    assert_eq!(cfg.schema_version, 1);
    assert_eq!(cfg.strategy.default_branch, "main");
    assert_eq!(
        cfg.branches.feature.pattern,
        "feature/{issue}-{slug}"
    );
}

#[test]
fn unknown_field_is_error() {
    let yaml = r#"
schema-version: 1
strategy:
  preset: github-flow
  default-branch: main
typo-field: true
"#;
    let err = parse_policy_yaml(yaml).unwrap_err();
    match err {
        WorkbenchError::InvalidPolicy { findings } => {
            assert!(findings.iter().any(|f| f.rule_id.contains("unknown")));
        }
        other => panic!("unexpected {other:?}"),
    }
}

#[test]
fn defaults_preset_fills_branch_patterns() {
    let cfg = github_flow_defaults();
    assert!(cfg.branches.feature.require_issue);
    assert_eq!(cfg.pull_requests.required_base, "main");
    assert!(cfg.pull_requests.draft_by_default);
}
```

- [ ] **Step 2: Run — expect FAIL**

Run: `cargo test -p workbench-domain --test policy_load`

Expected: FAIL (missing API).

- [ ] **Step 3: Implement schema + preset + load**

Use `serde` with `#[serde(deny_unknown_fields)]` on root and nested structs. Map kebab-case with `#[serde(rename_all = "kebab-case")]`.

Minimal `PolicyConfig` shape (match product design example subset):

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct PolicyConfig {
    pub schema_version: u32,
    pub strategy: StrategyConfig,
    #[serde(default)]
    pub branches: BranchesConfig,
    #[serde(default)]
    pub commits: CommitsConfig,
    #[serde(default)]
    pub pull_requests: PullRequestsConfig,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct StrategyConfig {
    pub preset: String,
    pub default_branch: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct BranchesConfig {
    #[serde(default = "default_feature")]
    pub feature: BranchTypeConfig,
    #[serde(default = "default_fix")]
    pub fix: BranchTypeConfig,
    #[serde(default = "default_prefixes")]
    pub allowed_prefixes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct BranchTypeConfig {
    pub pattern: String,
    pub start_from: String,
    pub require_issue: bool,
}

// CommitsConfig: require_signing: bool, conventional_commits: Enforcement (off|warning|blocker)
// PullRequestsConfig: draft_by_default, required_base, merge_method, require_linked_issue
```

`parse_policy_yaml`:

1. Parse with `serde_yaml::from_str::<PolicyConfig>`.
2. On serde error mentioning unknown field, map to `InvalidPolicy` with a finding `rule_id: "policy.unknown-field"`.
3. If `schema_version != 1`, `InvalidPolicy`.
4. If `strategy.preset != "github-flow"`, `InvalidPolicy`.
5. Start from `github_flow_defaults()`, then deserialize overlay: simplest approach for Phase 1 — deserialize full struct with defaults on nested fields via `#[serde(default)]` functions that match the preset.

`github_flow_defaults()` returns the complete config matching design §8.1 defaults for GitHub Flow.

Update `policy/mod.rs` to export `parse_policy_yaml`, `github_flow_defaults`, `PolicyConfig`, etc.

- [ ] **Step 4: Add round-trip property test in `load.rs`**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn defaults_round_trip_yaml() {
        let cfg = github_flow_defaults();
        let yaml = serde_yaml::to_string(&cfg).unwrap();
        let parsed = parse_policy_yaml(&yaml).unwrap();
        assert_eq!(parsed.schema_version, cfg.schema_version);
        assert_eq!(parsed.strategy, cfg.strategy);
        assert_eq!(parsed.branches.feature.pattern, cfg.branches.feature.pattern);
    }
}
```

- [ ] **Step 5: Run — expect PASS**

Run: `cargo test -p workbench-domain --test policy_load`
Run: `cargo test -p workbench-domain defaults_round_trip_yaml`

Expected: PASS

- [ ] **Step 6: Commit**

```bash
git add crates/workbench-domain
git commit -m "feat(domain): parse and validate policy schema v1"
```

---

### Task 5: Repository types and workflow state machine

**Files:**
- Create: `crates/workbench-domain/src/repository/mod.rs` (replace stub)
- Create: `crates/workbench-domain/src/workflow/state.rs`
- Create: `crates/workbench-domain/tests/workflow_state.rs`

**Interfaces:**
- Consumes: none for types; state machine is pure
- Produces:
  - `RepositoryId { owner, name }`, `Remote { name, url }`, `BranchState { … }`
  - `WorkflowState` enum matching design §11.2
  - `fn can_transition(from, to) -> bool`
  - `fn transition(from, to) -> Result<WorkflowState, WorkbenchError>`

- [ ] **Step 1: Write failing state tests**

```rust
use workbench_domain::workflow::state::{can_transition, transition, WorkflowState};

#[test]
fn happy_path_allows_unstarted_to_branch_created() {
    assert!(can_transition(
        WorkflowState::Unstarted,
        WorkflowState::BranchCreated
    ));
}

#[test]
fn rejects_skip_to_merged() {
    assert!(!can_transition(
        WorkflowState::Unstarted,
        WorkflowState::Merged
    ));
}

#[test]
fn transition_returns_new_state() {
    let next = transition(WorkflowState::Pushed, WorkflowState::PullRequestDraft).unwrap();
    assert_eq!(next, WorkflowState::PullRequestDraft);
}
```

- [ ] **Step 2: Run — expect FAIL**

Run: `cargo test -p workbench-domain --test workflow_state`

- [ ] **Step 3: Implement repository types**

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepositoryId {
    pub owner: String,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Remote {
    pub name: String,
    pub url: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BranchState {
    pub name: String,
    pub head_oid: Option<String>,
    pub upstream: Option<String>,
    pub base_branch: Option<String>,
    pub ahead: u64,
    pub behind: u64,
    pub dirty_paths: Vec<String>,
    pub is_protected: bool,
}
```

- [ ] **Step 4: Implement `WorkflowState`**

States: `Unstarted`, `BranchCreated`, `ChangesPresent`, `Committed`, `Pushed`, `PullRequestDraft`, `ValidationPending`, `ReviewPending`, `ReadyToMerge`, `Merged`, `CleanupPending`, `Complete`.

Allowed edges (linear happy path + stay for re-validation):

```text
Unstarted -> BranchCreated
BranchCreated -> ChangesPresent
ChangesPresent -> Committed
Committed -> Pushed
Pushed -> PullRequestDraft
PullRequestDraft -> ValidationPending
ValidationPending -> ReviewPending | ValidationPending  // rerun
ReviewPending -> ReadyToMerge
ReadyToMerge -> Merged
Merged -> CleanupPending
CleanupPending -> Complete
```

Also allow `BranchCreated -> Committed` if committing without separate "changes present" observation is desired — **stick to the linear list above** for Phase 1 clarity.

Map illegal transitions to `WorkbenchError::InvalidBranchName` is wrong — add:

```rust
#[error("illegal workflow transition from {from:?} to {to:?}")]
IllegalTransition { from: WorkflowState, to: WorkflowState },
```

Add this variant in `error.rs` as part of this task.

- [ ] **Step 5: Run — expect PASS**

Run: `cargo test -p workbench-domain --test workflow_state`

- [ ] **Step 6: Commit**

```bash
git add crates/workbench-domain
git commit -m "feat(domain): add repository types and workflow state machine"
```

---

### Task 6: Typed operation plans and CreateBranch planner

**Files:**
- Create: `crates/workbench-domain/src/operations/plan.rs`
- Create: `crates/workbench-domain/src/operations/create_branch.rs`
- Modify: `crates/workbench-domain/src/operations/mod.rs`
- Create: `crates/workbench-domain/tests/create_branch_plan.rs`
- Create snapshots via `insta` on first run

**Interfaces:**
- Consumes: `PolicyConfig`, `BranchState`, `normalize_slug` / `branch_name`, `WorkbenchError`, `PolicyFinding`
- Produces:
  - `RiskClass::{Low, Medium, High}`
  - `StepStatus::{Pending, Running, Succeeded, Failed, Skipped, CompensationNeeded}`
  - `GitCommand::{Fetch, CreateBranch { name, start_point }, PushRef { … }}`
  - `OperationPlan { id: Ulid, kind, risk, summary, rationale, commands, preconditions, findings }`
  - `fn plan_create_branch_from_issue(policy: &PolicyConfig, issue: u64, title: &str, current: &BranchState) -> Result<OperationPlan, WorkbenchError>`

- [ ] **Step 1: Write failing planner test + golden**

```rust
use workbench_domain::operations::create_branch::plan_create_branch_from_issue;
use workbench_domain::policy::github_flow_defaults;
use workbench_domain::repository::BranchState;

#[test]
fn plans_feature_branch_for_issue_42() {
    let policy = github_flow_defaults();
    let current = BranchState {
        name: "main".into(),
        head_oid: Some("abc".into()),
        upstream: Some("origin/main".into()),
        base_branch: Some("main".into()),
        ahead: 0,
        behind: 0,
        dirty_paths: vec![],
        is_protected: true,
    };
    let plan = plan_create_branch_from_issue(&policy, 42, "Add resumable uploads", &current)
        .unwrap();
    assert!(plan.summary.contains("feature/42-add-resumable-uploads"));
    assert!(matches!(
        plan.risk,
        workbench_domain::operations::plan::RiskClass::Low
    ));

    // Stabilize id for snapshot
    let mut stable = plan.clone();
    stable.id = ulid::Ulid::nil();
    insta::assert_yaml_snapshot!("create_branch_issue_42", stable);
}

#[test]
fn blocked_when_require_issue_and_issue_zero_not_applicable() {
    // issue 0 might be invalid — treat issue must be >= 1
    let policy = github_flow_defaults();
    let current = BranchState {
        name: "main".into(),
        head_oid: Some("abc".into()),
        upstream: None,
        base_branch: Some("main".into()),
        ahead: 0,
        behind: 0,
        dirty_paths: vec![],
        is_protected: true,
    };
    let err = plan_create_branch_from_issue(&policy, 0, "Nope", &current).unwrap_err();
    assert!(matches!(
        err,
        workbench_domain::WorkbenchError::InvalidBranchName { .. }
            | workbench_domain::WorkbenchError::PolicyBlocked { .. }
    ));
}
```

- [ ] **Step 2: Run — expect FAIL**

Run: `cargo test -p workbench-domain --test create_branch_plan`

- [ ] **Step 3: Implement plan types**

```rust
use serde::{Deserialize, Serialize};
use ulid::Ulid;
use crate::policy::PolicyFinding;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RiskClass {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum StepStatus {
    Pending,
    Running,
    Succeeded,
    Failed,
    Skipped,
    CompensationNeeded,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum GitCommand {
    Fetch { remote: String },
    CreateBranch { name: String, start_point: String },
    PushRef {
        remote: String,
        local_ref: String,
        remote_ref: String,
        set_upstream: bool,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OperationPlan {
    pub id: Ulid,
    pub kind: String,
    pub risk: RiskClass,
    pub summary: String,
    pub rationale: Vec<String>,
    pub commands: Vec<GitCommand>,
    pub preconditions: Vec<String>,
    pub findings: Vec<PolicyFinding>,
}
```

- [ ] **Step 4: Implement `plan_create_branch_from_issue`**

Logic:

1. If `issue == 0`, return `InvalidBranchName { reason: "issue number must be >= 1" }`.
2. If `policy.branches.feature.require_issue` (always true in defaults), proceed.
3. `name = branch_name(&policy.branches.feature.pattern, issue, title)?`.
4. `start_point = policy.branches.feature.start_from` (e.g. `main`).
5. If `current.name == start_point` is false, still allow planning but add rationale that execution should fetch/checkout base first — include `GitCommand::Fetch { remote: "origin".into() }` then `CreateBranch`.
6. If caller tries to use protected branch as the **new** branch name (name == default branch), return `ProtectedBranchMisuse`.
7. Risk: `Low`.
8. Summary: `Create branch {name} from {start_point} for issue #{issue}`.
9. Rationale bullets: policy pattern, require-issue, start-from.
10. `id: Ulid::new()` (tests overwrite to nil for snapshots).

- [ ] **Step 5: Run tests; accept snapshots**

Run: `cargo test -p workbench-domain --test create_branch_plan`
Run: `cargo insta accept -p workbench-domain` if needed (install `cargo-insta` or use `INSTA_UPDATE=1`).

Expected: PASS with snapshot file under `crates/workbench-domain/tests/snapshots/`.

- [ ] **Step 6: Commit**

```bash
git add crates/workbench-domain
git commit -m "feat(domain): add operation plans and create-branch planner"
```

---

### Task 7: CI workflow and final verification

**Files:**
- Create: `.github/workflows/ci.yml`
- Modify: none unless clippy/fmt fixes required

**Interfaces:**
- Consumes: full workspace
- Produces: green local checks matching CI

- [ ] **Step 1: Add CI workflow**

```yaml
name: ci

on:
  push:
  pull_request:

jobs:
  rust:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          components: rustfmt, clippy
      - name: Format
        run: cargo fmt --all -- --check
      - name: Clippy
        run: cargo clippy --workspace --all-targets -- -D warnings
      - name: Test
        run: cargo test --workspace
```

- [ ] **Step 2: Run local gates**

Run:

```bash
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo run -p workbench-cli -- --version
```

Expected: all succeed; `gww 0.1.0`.

- [ ] **Step 3: Commit**

```bash
git add .github/workflows/ci.yml crates
git commit -m "ci: add fmt, clippy, and test workflow"
```

- [ ] **Step 4: Exit-criterion checklist**

Confirm manually:

- [ ] Dual license files present
- [ ] Product design copied under `docs/product/`
- [ ] Domain has policy parse, branch naming, workflow transitions, create-branch plans
- [ ] Property + golden tests exist and pass
- [ ] Stub crates compile; `gww --version` works
- [ ] No Tauri/React; domain has no Git/GitHub I/O

---

## Self-review (plan vs spec)

| Spec requirement | Task |
|---|---|
| Dual MIT/Apache license + OSS docs | Task 1 |
| Copy product design + architecture doc | Task 1 |
| Cargo workspace + stub adapters/CLI | Task 1 |
| Policy schema v1, unknown fields error, GitHub Flow preset | Task 4 |
| Repository/branch types | Task 5 |
| Branch naming / slug rules | Task 3 |
| Workflow state enum + transitions | Task 5 |
| Typed operation plans + CreateBranch | Task 6 |
| Unit / proptest / insta tests | Tasks 2–6 |
| `gww` stub | Task 1 |
| Application ports only | Task 1 |
| CI fmt/clippy/test | Task 7 |
| No Tauri / no real git/gh/sqlite | Global + Tasks 1–7 |

No intentional placeholders left; ULID chosen; product design path fixed to `docs/product/`.
