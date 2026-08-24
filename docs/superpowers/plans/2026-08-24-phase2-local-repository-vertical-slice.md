# Phase 2 Local Repository Vertical Slice Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Deliver a CLI-first `gww` vertical slice that opens a real local Git repository, creates a policy-compliant feature branch from a manually entered issue number and title, previews and executes a push to a configured remote, and journals every multi-step operation in SQLite.

**Architecture:** Thin `workbench-cli` handlers call `workbench-application` use cases. Use cases orchestrate Phase 1 domain planners plus new `plan_push`, talk only to ports (`GitClient`, `OperationStore`, `ProcessRunner`, `Clock`, `IdGenerator`, `PolicySource`), and never shell-interpolate. `workbench-git` is a process-based Git adapter (program + argv). `workbench-storage` is SQLite with migrations. `workbench-github` stays a stub.

**Tech Stack:** Rust 2021, `clap` 4, `rusqlite` (bundled), `serde_json`, `thiserror`, `ulid`, `time`, `tempfile` (tests). No Tokio, no `libgit2`, no Tauri/React, no `gh`.

## Global Constraints

- Spec: `docs/superpowers/specs/2026-08-23-phase2-local-repository-vertical-slice-design.md`.
- Product design: `docs/product/GITHUB_WORKFLOW_WORKBENCH_DESIGN.md` (§7.1–7.3, §12–14, §16, §18, §24 Phase 2).
- CLI binary name: `gww`.
- No Tauri / React. No GitHub API / `gh`. No force push (`--force` / `--force-with-lease` must never appear in argv).
- Git execution is program + `Vec<String>` argv only (no shell strings).
- Domain crate remains free of filesystem, Git, GitHub, SQLite, Tokio, reqwest, and Tauri.
- `workbench-application` depends on `workbench-domain` only (plus serde/thiserror/ulid/time). It must not depend on `workbench-git` or `workbench-storage`.
- `workbench-git` and `workbench-storage` depend on `workbench-application` to implement ports.
- Default DB path: `$GWW_DATA_DIR/workbench.db` if set; else `$XDG_DATA_HOME/github-workbench/workbench.db` on Unix; else `$HOME/.local/share/github-workbench/workbench.db`; on Windows `$LOCALAPPDATA/github-workbench/workbench.db`.
- Exit codes: `0` success, `1` failure, `2` invalid usage/config (including invalid policy file), `3` policy blockers. Auth `4` unused.
- Invalid `.github-workbench.yml` is an error and must not mutate Git or SQLite.
- Push is blocked when the working tree is dirty.
- Do not assume the remote is named `origin`.
- Prefer TDD: failing test → implement → pass → commit per task.
- On Windows PowerShell, use `git commit -m "message"` (no bash heredoc required).

---

## File structure

```text
Cargo.toml                                          # workspace deps: clap, rusqlite, serde_json, tempfile, time
docs/architecture.md                                # Phase 2 layering
README.md                                           # Phase 2 CLI status
crates/workbench-domain/src/repository/mod.rs        # + RepositorySnapshot, RemoteIdentity, parse_github_remote
crates/workbench-domain/src/policy/evaluate.rs       # + evaluate_current_branch_policy
crates/workbench-domain/src/operations/plan.rs       # + GitCommand::step_kind
crates/workbench-domain/src/operations/create_branch.rs  # remote parameter
crates/workbench-domain/src/operations/push.rs       # plan_push
crates/workbench-domain/src/operations/mod.rs
crates/workbench-domain/tests/create_branch_plan.rs  # update call sites
crates/workbench-domain/tests/push_plan.rs
crates/workbench-domain/tests/remote_parse.rs
crates/workbench-domain/tests/branch_policy.rs
crates/workbench-domain/tests/snapshots/push_plan__push_new_feature_branch.snap
crates/workbench-application/Cargo.toml
crates/workbench-application/src/lib.rs
crates/workbench-application/src/error.rs
crates/workbench-application/src/ports.rs
crates/workbench-application/src/clock.rs
crates/workbench-application/src/ids.rs
crates/workbench-application/src/redact.rs
crates/workbench-application/src/remote.rs
crates/workbench-application/src/policy_source.rs
crates/workbench-application/src/recommend.rs
crates/workbench-application/src/executor.rs
crates/workbench-application/src/fakes.rs
crates/workbench-application/src/use_cases/mod.rs
crates/workbench-application/src/use_cases/open.rs
crates/workbench-application/src/use_cases/status.rs
crates/workbench-application/src/use_cases/start_issue.rs
crates/workbench-application/src/use_cases/push.rs
crates/workbench-application/src/use_cases/ops.rs
crates/workbench-application/tests/open_and_status.rs
crates/workbench-application/tests/start_issue.rs
crates/workbench-application/tests/push.rs
crates/workbench-git/Cargo.toml
crates/workbench-git/README.md
crates/workbench-git/src/lib.rs
crates/workbench-git/src/argv.rs
crates/workbench-git/src/env.rs
crates/workbench-git/src/process.rs
crates/workbench-git/src/parser.rs
crates/workbench-git/src/client.rs
crates/workbench-git/tests/parser.rs
crates/workbench-git/tests/git_integration.rs
crates/workbench-storage/Cargo.toml
crates/workbench-storage/README.md
crates/workbench-storage/src/lib.rs
crates/workbench-storage/src/sqlite.rs
crates/workbench-storage/src/migrations.rs
crates/workbench-storage/src/migrations/001_initial.sql
crates/workbench-storage/tests/sqlite_store.rs
crates/workbench-cli/Cargo.toml
crates/workbench-cli/src/main.rs
crates/workbench-cli/src/args.rs
crates/workbench-cli/src/data_dir.rs
crates/workbench-cli/src/render.rs
crates/workbench-cli/src/confirm.rs
crates/workbench-cli/tests/cli_happy_path.rs
```

---

### Task 1: Domain snapshot, remote parse, push planner, create-branch remote

**Files:**
- Modify: `crates/workbench-domain/src/repository/mod.rs`
- Modify: `crates/workbench-domain/src/operations/plan.rs`
- Modify: `crates/workbench-domain/src/operations/create_branch.rs`
- Modify: `crates/workbench-domain/src/operations/mod.rs`
- Modify: `crates/workbench-domain/src/policy/evaluate.rs`
- Modify: `crates/workbench-domain/src/policy/mod.rs`
- Modify: `crates/workbench-domain/tests/create_branch_plan.rs`
- Create: `crates/workbench-domain/src/operations/push.rs`
- Create: `crates/workbench-domain/tests/push_plan.rs`
- Create: `crates/workbench-domain/tests/remote_parse.rs`

**Interfaces:**
- Consumes: `PolicyConfig`, `BranchState`, `Remote`, `GitCommand`, `OperationPlan`, `WorkbenchError`, `PolicyFinding`, `Severity`
- Produces:
  - `RepositorySnapshot { root, branch, detached_head, head_oid, dirty_paths, remotes, selected_remote, upstream }`
  - `RemoteIdentity { host, owner, name }`
  - `fn parse_github_remote(url: &str) -> Option<RemoteIdentity>`
  - `fn evaluate_current_branch_policy(policy: &PolicyConfig, branch: &str) -> Vec<PolicyFinding>`
  - `GitCommand::step_kind(&self) -> &'static str`
  - `fn plan_create_branch_from_issue(policy, issue, title, current, remote) -> Result<OperationPlan, WorkbenchError>`
  - `fn plan_push(policy, current, remote) -> Result<OperationPlan, WorkbenchError>`

- [ ] **Step 1: Write failing tests**

`crates/workbench-domain/tests/remote_parse.rs`:

```rust
use workbench_domain::repository::parse_github_remote;

#[test]
fn parses_ssh_scp_syntax() {
    let id = parse_github_remote("git@github.com:acme/widgets.git").unwrap();
    assert_eq!(id.host, "github.com");
    assert_eq!(id.owner, "acme");
    assert_eq!(id.name, "widgets");
}

#[test]
fn parses_https() {
    let id = parse_github_remote("https://github.com/acme/widgets.git").unwrap();
    assert_eq!(id.owner, "acme");
    assert_eq!(id.name, "widgets");
}

#[test]
fn parses_ssh_url() {
    let id = parse_github_remote("ssh://git@github.com/acme/widgets.git").unwrap();
    assert_eq!(id.owner, "acme");
    assert_eq!(id.name, "widgets");
}

#[test]
fn rejects_non_githubish_path() {
    assert!(parse_github_remote("/tmp/local-bare.git").is_none());
}
```

`crates/workbench-domain/tests/push_plan.rs`:

```rust
use workbench_domain::operations::plan::{GitCommand, RiskClass};
use workbench_domain::operations::push::plan_push;
use workbench_domain::policy::github_flow_defaults;
use workbench_domain::repository::BranchState;
use workbench_domain::WorkbenchError;

fn feature_branch(ahead: u64, upstream: Option<&str>) -> BranchState {
    BranchState {
        name: "feature/42-add-resumable-uploads".into(),
        head_oid: Some("abc".into()),
        upstream: upstream.map(str::to_string),
        base_branch: Some("main".into()),
        ahead,
        behind: 0,
        dirty_paths: vec![],
        is_protected: false,
    }
}

#[test]
fn plans_low_risk_push_for_new_upstream() {
    let policy = github_flow_defaults();
    let current = feature_branch(2, None);
    let plan = plan_push(&policy, &current, "github").unwrap();
    assert_eq!(plan.kind, "push");
    assert_eq!(plan.risk, RiskClass::Low);
    assert!(plan.summary.contains("github"));
    assert!(matches!(
        &plan.commands[..],
        [
            GitCommand::Fetch { remote },
            GitCommand::PushRef {
                remote: push_remote,
                local_ref,
                remote_ref,
                set_upstream: true,
            }
        ] if remote == "github"
            && push_remote == "github"
            && local_ref == "feature/42-add-resumable-uploads"
            && remote_ref == "feature/42-add-resumable-uploads"
    ));
    let mut stable = plan.clone();
    stable.id = ulid::Ulid::nil();
    insta::assert_yaml_snapshot!("push_new_feature_branch", stable);
}

#[test]
fn nothing_to_push_when_ahead_is_zero() {
    let policy = github_flow_defaults();
    let plan = plan_push(&policy, &feature_branch(0, Some("github/feature/42-add-resumable-uploads")), "github").unwrap();
    assert!(plan.commands.is_empty());
    assert!(plan.summary.contains("Nothing to push"));
}

#[test]
fn refuses_push_of_default_branch() {
    let policy = github_flow_defaults();
    let current = BranchState {
        name: "main".into(),
        head_oid: Some("abc".into()),
        upstream: Some("github/main".into()),
        base_branch: Some("main".into()),
        ahead: 1,
        behind: 0,
        dirty_paths: vec![],
        is_protected: true,
    };
    let err = plan_push(&policy, &current, "github").unwrap_err();
    assert!(matches!(err, WorkbenchError::ProtectedBranchMisuse { branch } if branch == "main"));
}

#[test]
fn existing_upstream_is_medium_risk() {
    let policy = github_flow_defaults();
    let plan = plan_push(
        &policy,
        &feature_branch(1, Some("github/feature/42-add-resumable-uploads")),
        "github",
    )
    .unwrap();
    assert_eq!(plan.risk, RiskClass::Medium);
    assert!(matches!(
        plan.commands.last(),
        Some(GitCommand::PushRef { set_upstream: false, .. })
    ));
}
```

Update `crates/workbench-domain/tests/create_branch_plan.rs` so every `plan_create_branch_from_issue` call passes `"origin"` as the last argument, and add:

```rust
#[test]
fn fetch_uses_provided_remote_not_origin() {
    let policy = github_flow_defaults();
    let current = branch_state("topic");
    let plan = plan_create_branch_from_issue(&policy, 42, "Add resumable uploads", &current, "github")
        .unwrap();
    assert!(matches!(
        plan.commands.first(),
        Some(workbench_domain::operations::plan::GitCommand::Fetch { remote }) if remote == "github"
    ));
}
```

Create `crates/workbench-domain/tests/branch_policy.rs`:

```rust
use workbench_domain::policy::{evaluate_current_branch_policy, github_flow_defaults, Severity};

#[test]
fn allowed_prefix_is_silent() {
    let policy = github_flow_defaults();
    let findings = evaluate_current_branch_policy(&policy, "feature/42-add-resumable-uploads");
    assert!(findings.is_empty());
}

#[test]
fn default_branch_is_silent() {
    let policy = github_flow_defaults();
    assert!(evaluate_current_branch_policy(&policy, "main").is_empty());
}

#[test]
fn unknown_prefix_is_warning() {
    let policy = github_flow_defaults();
    let findings = evaluate_current_branch_policy(&policy, "wip/experiment");
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].rule_id, "branches.allowed-prefixes");
    assert_eq!(findings[0].severity, Severity::Warning);
}
```

- [ ] **Step 2: Run tests — expect FAIL**

Run: `cargo test -p workbench-domain --test remote_parse --test push_plan --test create_branch_plan --test branch_policy`

Expected: FAIL (missing types/functions or extra-arg compile errors).

- [ ] **Step 3: Implement domain types and planners**

Append to `repository/mod.rs`:

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemoteIdentity {
    pub host: String,
    pub owner: String,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepositorySnapshot {
    pub root: String,
    pub branch: Option<String>,
    pub detached_head: bool,
    pub head_oid: Option<String>,
    pub dirty_paths: Vec<String>,
    pub remotes: Vec<Remote>,
    pub selected_remote: Option<String>,
    pub upstream: Option<String>,
}

pub fn parse_github_remote(url: &str) -> Option<RemoteIdentity> {
    let url = url.trim();
    let url = url.strip_suffix(".git").unwrap_or(url);

    if let Some(rest) = url.strip_prefix("git@") {
        let (host, path) = rest.split_once(':')?;
        return identity_from_host_path(host, path);
    }
    if let Some(rest) = url.strip_prefix("ssh://") {
        let rest = rest.strip_prefix("git@").unwrap_or(rest);
        let (host, path) = rest.split_once('/')?;
        let host = host.split_once('@').map(|(_, h)| h).unwrap_or(host);
        return identity_from_host_path(host, path);
    }
    if let Some(rest) = url.strip_prefix("https://") {
        let rest = rest.strip_prefix("git@").unwrap_or(rest);
        let (host, path) = rest.split_once('/')?;
        return identity_from_host_path(host, path);
    }
    if let Some(rest) = url.strip_prefix("http://") {
        let (host, path) = rest.split_once('/')?;
        return identity_from_host_path(host, path);
    }
    None
}

fn identity_from_host_path(host: &str, path: &str) -> Option<RemoteIdentity> {
    let path = path.trim_start_matches('/');
    let (owner, name) = path.split_once('/')?;
    if owner.is_empty() || name.is_empty() || name.contains('/') {
        return None;
    }
    Some(RemoteIdentity {
        host: host.to_string(),
        owner: owner.to_string(),
        name: name.to_string(),
    })
}
```

Add `GitCommand::step_kind` to `plan.rs`:

```rust
impl GitCommand {
    pub fn step_kind(&self) -> &'static str {
        match self {
            GitCommand::Fetch { .. } => "fetch",
            GitCommand::CreateBranch { .. } => "create-branch",
            GitCommand::PushRef { .. } => "push-ref",
        }
    }
}
```

Change `plan_create_branch_from_issue` signature to take `remote: &str` and use it in `GitCommand::Fetch { remote: remote.into() }` instead of `"origin"`.

Create `operations/push.rs`:

```rust
use super::plan::{GitCommand, OperationPlan, RiskClass};
use crate::policy::PolicyConfig;
use crate::repository::BranchState;
use crate::WorkbenchError;
use ulid::Ulid;

pub fn plan_push(
    policy: &PolicyConfig,
    current: &BranchState,
    remote: &str,
) -> Result<OperationPlan, WorkbenchError> {
    if current.name == policy.strategy.default_branch {
        return Err(WorkbenchError::ProtectedBranchMisuse {
            branch: current.name.clone(),
        });
    }

    if current.ahead == 0 {
        return Ok(OperationPlan {
            id: Ulid::new(),
            kind: "push".into(),
            risk: RiskClass::Low,
            summary: format!(
                "Nothing to push: `{}` has no commits ahead of its comparison ref.",
                current.name
            ),
            rationale: vec![
                "Phase 2 pushes only commits that already exist on the current feature branch."
                    .into(),
                format!("Ahead count is {}.", current.ahead),
            ],
            commands: vec![],
            preconditions: vec!["Working tree is clean.".into()],
            findings: vec![],
        });
    }

    let set_upstream = current.upstream.is_none();
    let risk = if set_upstream {
        RiskClass::Low
    } else {
        RiskClass::Medium
    };
    let upstream_reason = match &current.upstream {
        None => "No upstream is set; the push will create the remote branch and set upstream.".into(),
        Some(upstream) => format!(
            "Upstream is `{upstream}`; this updates the existing remote feature branch."
        ),
    };

    Ok(OperationPlan {
        id: Ulid::new(),
        kind: "push".into(),
        risk,
        summary: format!("Push `{}` to `{}/{}`", current.name, remote, current.name),
        rationale: vec![
            format!("Remote `{remote}` is the selected push target."),
            upstream_reason,
            "Force push is never used.".into(),
        ],
        commands: vec![
            GitCommand::Fetch {
                remote: remote.into(),
            },
            GitCommand::PushRef {
                remote: remote.into(),
                local_ref: current.name.clone(),
                remote_ref: current.name.clone(),
                set_upstream,
            },
        ],
        preconditions: vec![
            "Working tree is clean.".into(),
            format!("Local branch `{}` exists.", current.name),
            format!("Remote `{remote}` is reachable."),
        ],
        findings: vec![],
    })
}
```

`operations/mod.rs`:

```rust
pub mod create_branch;
pub mod plan;
pub mod push;
```

Add to `policy/evaluate.rs`:

```rust
pub fn evaluate_current_branch_policy(policy: &PolicyConfig, branch: &str) -> Vec<PolicyFinding> {
    if branch == policy.strategy.default_branch {
        return Vec::new();
    }
    let allowed = &policy.branches.allowed_prefixes;
    let matches_prefix = allowed.iter().any(|prefix| {
        branch == prefix.as_str() || branch.starts_with(&format!("{prefix}/"))
    });
    if matches_prefix {
        return Vec::new();
    }
    vec![PolicyFinding {
        rule_id: "branches.allowed-prefixes".into(),
        severity: Severity::Warning,
        expected: allowed.join(", "),
        actual: branch.into(),
        message: "Current branch does not use an allowed prefix.".into(),
        remediation: "Rename the branch to match repository policy, or start a new issue branch."
            .into(),
    }]
}
```

Export `evaluate_current_branch_policy` from `policy/mod.rs`.

- [ ] **Step 4: Run tests — expect PASS**

Run:

```bash
cargo test -p workbench-domain --test remote_parse --test push_plan --test create_branch_plan
cargo test -p workbench-domain evaluate_current_branch
INSTA_UPDATE=1 cargo test -p workbench-domain --test push_plan
```

Expected: PASS, with `crates/workbench-domain/tests/snapshots/push_plan__push_new_feature_branch.snap` created. Re-run without `INSTA_UPDATE` to confirm.

- [ ] **Step 5: Commit**

```bash
git add crates/workbench-domain
git commit -m "feat(domain): add repository snapshot, remote parse, and push planner"
```

---

### Task 2: Application errors, ports, and fakes

**Files:**
- Modify: `crates/workbench-application/Cargo.toml`
- Modify: `crates/workbench-application/src/lib.rs`
- Modify: `crates/workbench-application/src/ports.rs`
- Modify: `Cargo.toml` (workspace deps)
- Create: `crates/workbench-application/src/error.rs`
- Create: `crates/workbench-application/src/clock.rs`
- Create: `crates/workbench-application/src/ids.rs`
- Create: `crates/workbench-application/src/redact.rs`
- Create: `crates/workbench-application/src/remote.rs`
- Create: `crates/workbench-application/src/policy_source.rs`
- Create: `crates/workbench-application/src/fakes.rs`
- Create: unit tests in `error.rs`, `redact.rs`, `remote.rs`

**Interfaces:**
- Consumes: `WorkbenchError`, `PolicyFinding`, `StepStatus`, `GitCommand`, `OperationPlan`, `RepositorySnapshot`, `BranchState`, `Remote`, `Ulid`
- Produces:
  - `AppError` variants listed below, plus `exit_code()` and `user_report()`
  - `CommandSpec`, `CommandOutput`
  - traits `ProcessRunner`, `GitClient`, `OperationStore`, `Clock`, `IdGenerator`, `PolicySource`
  - records `ProjectRecord`, `OperationRecord`, `StepRecord`
  - `fn resolve_remote(remotes, mapped_remote, flag) -> Result<String, AppError>`
  - `fn redact(text: &str) -> String`
  - `fn bound_output(text: &str) -> String` (64 KiB max)
  - `fakes::{FakeClock, FakeIds, FakeGit, FakeStore, FakePolicy}`

- [ ] **Step 1: Add workspace dependencies**

In root `Cargo.toml` `[workspace.dependencies]`:

```toml
serde_json = "1"
clap = { version = "4", features = ["derive"] }
rusqlite = { version = "0.32", features = ["bundled"] }
tempfile = "3"
time = { version = "0.3", features = ["formatting", "parsing", "std"] }
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
serde = { workspace = true }
serde_json = { workspace = true }
thiserror = { workspace = true }
ulid = { workspace = true }
time = { workspace = true }

[dev-dependencies]
pretty_assertions = { workspace = true }
tempfile = { workspace = true }
```

- [ ] **Step 2: Write failing tests for redact, remote resolution, and error exit codes**

`redact.rs` tests:

```rust
#[test]
fn redacts_github_pat_and_basic_auth() {
    let input = "https://x-access-token:ghp_abcdefghijklmnopqrstuvwxyz012345@github.com/acme/widgets.git\nAuthorization: Bearer gho_abcdefghijklmnopqrstuvwxyz012345";
    let out = redact(input);
    assert!(!out.contains("ghp_abcdefghijklmnopqrstuvwxyz012345"));
    assert!(!out.contains("gho_abcdefghijklmnopqrstuvwxyz012345"));
    assert!(out.contains("[redacted]"));
}

#[test]
fn truncates_long_output() {
    let huge = "a".repeat(70_000);
    let out = bound_output(&huge);
    assert!(out.len() < 70_000);
    assert!(out.contains("[truncated]"));
}
```

`remote.rs` tests:

```rust
fn remotes(names: &[&str]) -> Vec<Remote> {
    names
        .iter()
        .map(|name| Remote {
            name: (*name).into(),
            url: format!("git@github.com:acme/{name}.git"),
        })
        .collect()
}

#[test]
fn flag_wins() {
    assert_eq!(
        resolve_remote(&remotes(&["origin", "github"]), Some("origin"), Some("github")).unwrap(),
        "github"
    );
}

#[test]
fn mapped_used_when_no_flag() {
    assert_eq!(
        resolve_remote(&remotes(&["origin", "github"]), Some("github"), None).unwrap(),
        "github"
    );
}

#[test]
fn sole_remote_used() {
    assert_eq!(
        resolve_remote(&remotes(&["github"]), None, None).unwrap(),
        "github"
    );
}

#[test]
fn multiple_unmapped_is_error() {
    let err = resolve_remote(&remotes(&["origin", "github"]), None, None).unwrap_err();
    assert!(matches!(err, AppError::RemoteNotResolved { .. }));
}

#[test]
fn no_remotes_is_not_mapped() {
    let err = resolve_remote(&[], None, None).unwrap_err();
    assert!(matches!(err, AppError::RepositoryNotMapped));
}
```

`error.rs` test:

```rust
#[test]
fn policy_blocked_uses_exit_code_3() {
    let err = AppError::Domain(WorkbenchError::PolicyBlocked { findings: vec![] });
    assert_eq!(err.exit_code(), 3);
}

#[test]
fn invalid_policy_uses_exit_code_2() {
    let err = AppError::Domain(WorkbenchError::InvalidPolicy { findings: vec![] });
    assert_eq!(err.exit_code(), 2);
}

#[test]
fn git_failed_uses_exit_code_1() {
    let err = AppError::GitFailed {
        program: "git".into(),
        args_summary: "push github feature/x:feature/x".into(),
        status: 1,
        stderr_redacted: "rejected".into(),
    };
    assert_eq!(err.exit_code(), 1);
    let report = err.user_report();
    assert!(report.contains("What failed"));
    assert!(report.contains("retry"));
}
```

- [ ] **Step 3: Run — expect FAIL**

Run: `cargo test -p workbench-application`

Expected: FAIL (crate still stub / missing modules).

- [ ] **Step 4: Implement error, ports, clock, ids, redact, remote, policy source, fakes**

`error.rs`:

```rust
use std::fmt::{Write as _};
use thiserror::Error;
use workbench_domain::WorkbenchError;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum AppError {
    #[error(transparent)]
    Domain(#[from] WorkbenchError),

    #[error("git is not available: {detail}")]
    GitUnavailable { detail: String },

    #[error("git command failed: {program} {args_summary} (exit {status}): {stderr_redacted}")]
    GitFailed {
        program: String,
        args_summary: String,
        status: i32,
        stderr_redacted: String,
    },

    #[error("working tree is dirty ({0} path(s))", paths.len())]
    DirtyWorkingTree { paths: Vec<String> },

    #[error("could not resolve a unique remote")]
    RemoteNotResolved { candidates: Vec<String> },

    #[error("repository is not mapped to a Git remote")]
    RepositoryNotMapped,

    #[error("not a git repository: {path}")]
    NotAGitRepository { path: String },

    #[error("storage error: {detail}")]
    Storage { detail: String },

    #[error("I/O error at {path}: {detail}")]
    Io { path: String, detail: String },

    #[error("{message}")]
    Usage { message: String },

    #[error("{message}")]
    OperationFailed {
        message: String,
        changed: Vec<String>,
        unchanged: Vec<String>,
        retry_safe: bool,
        remediation: String,
    },
}

impl AppError {
    pub fn exit_code(&self) -> i32 {
        match self {
            AppError::Domain(WorkbenchError::PolicyBlocked { .. }) => 3,
            AppError::Domain(WorkbenchError::InvalidPolicy { .. })
            | AppError::Usage { .. }
            | AppError::Io { .. } => 2,
            _ => 1,
        }
    }

    pub fn user_report(&self) -> String {
        let (failed, changed, unchanged, retry_safe, remediation) = match self {
            AppError::OperationFailed {
                message,
                changed,
                unchanged,
                retry_safe,
                remediation,
            } => (
                message.clone(),
                changed.clone(),
                unchanged.clone(),
                *retry_safe,
                remediation.clone(),
            ),
            AppError::DirtyWorkingTree { paths } => (
                "Push is blocked because the working tree is dirty.".into(),
                Vec::new(),
                vec!["No Git refs were updated.".into()],
                true,
                format!(
                    "Commit or stash these paths, then retry: {}",
                    paths.join(", ")
                ),
            ),
            AppError::GitFailed {
                program,
                args_summary,
                status,
                stderr_redacted,
            } => (
                format!("{program} {args_summary} exited {status}: {stderr_redacted}"),
                Vec::new(),
                vec!["Later plan steps were not started.".into()],
                false,
                "Inspect the Git error, fix the repository state, then rerun the command.".into(),
            ),
            AppError::GitUnavailable { detail } => (
                format!("The git executable is not available ({detail})."),
                Vec::new(),
                vec!["No repository changes were made.".into()],
                true,
                "Install Git and ensure it is on PATH, or set GWW_GIT_PROGRAM.".into(),
            ),
            AppError::RemoteNotResolved { candidates } => (
                "Could not choose a unique Git remote.".into(),
                Vec::new(),
                vec!["No repository changes were made.".into()],
                true,
                format!(
                    "Pass --remote <name>. Candidates: {}",
                    candidates.join(", ")
                ),
            ),
            AppError::RepositoryNotMapped => (
                "This repository is not mapped to a Git remote / local project.".into(),
                Vec::new(),
                vec!["No repository changes were made.".into()],
                true,
                "Run `gww open <path>` (with --remote if several remotes exist).".into(),
            ),
            AppError::Domain(inner) => (
                inner.to_string(),
                Vec::new(),
                vec!["No repository changes were made.".into()],
                true,
                "Fix the policy or branch name and retry.".into(),
            ),
            other => (
                other.to_string(),
                Vec::new(),
                vec!["No repository changes were made.".into()],
                true,
                "See the error message above.".into(),
            ),
        };

        let mut out = String::new();
        let _ = writeln!(out, "What failed: {failed}");
        let _ = writeln!(
            out,
            "What already changed: {}",
            if changed.is_empty() {
                "nothing".into()
            } else {
                changed.join("; ")
            }
        );
        let _ = writeln!(
            out,
            "What did not happen: {}",
            if unchanged.is_empty() {
                "n/a".into()
            } else {
                unchanged.join("; ")
            }
        );
        let _ = writeln!(
            out,
            "Retry is safe: {}",
            if retry_safe { "yes" } else { "no, inspect journal first" }
        );
        let _ = writeln!(out, "Remediation: {remediation}");
        out
    }
}
```

`ports.rs` (replace the stub):

```rust
use std::path::{Path, PathBuf};

use crate::error::AppError;
use workbench_domain::operations::plan::{GitCommand, OperationPlan, StepStatus};
use workbench_domain::repository::{BranchState, Remote, RepositorySnapshot};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandSpec {
    pub program: String,
    pub args: Vec<String>,
    pub cwd: PathBuf,
    pub env: Vec<(String, String)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandOutput {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
}

pub trait ProcessRunner {
    fn run(&self, spec: &CommandSpec) -> Result<CommandOutput, AppError>;
}

pub trait GitClient {
    fn resolve_toplevel(&self, path: &Path) -> Result<PathBuf, AppError>;
    fn snapshot(&self, repo_root: &Path) -> Result<RepositorySnapshot, AppError>;
    fn branch_state(&self, repo_root: &Path, comparison_ref: &str) -> Result<BranchState, AppError>;
    fn list_remotes(&self, repo_root: &Path) -> Result<Vec<Remote>, AppError>;
    fn fetch(&self, repo_root: &Path, remote: &str) -> Result<CommandOutput, AppError>;
    fn create_branch(
        &self,
        repo_root: &Path,
        name: &str,
        start_point: &str,
    ) -> Result<CommandOutput, AppError>;
    fn push_ref(
        &self,
        repo_root: &Path,
        remote: &str,
        local_ref: &str,
        remote_ref: &str,
        set_upstream: bool,
    ) -> Result<CommandOutput, AppError>;
}

pub trait Clock {
    fn now_rfc3339(&self) -> String;
}

pub trait IdGenerator {
    fn next(&self) -> ulid::Ulid;
}

pub trait PolicySource {
    fn read_yaml(&self, repo_root: &Path) -> Result<Option<String>, AppError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectRecord {
    pub id: String,
    pub local_path: String,
    pub github_host: Option<String>,
    pub owner: Option<String>,
    pub repo: Option<String>,
    pub remote_name: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StepRecord {
    pub id: String,
    pub operation_id: String,
    pub sequence: i32,
    pub kind: String,
    pub status: StepStatus,
    pub detail_json: Option<String>,
    pub output_text: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperationRecord {
    pub id: String,
    pub project_id: String,
    pub kind: String,
    pub status: String,
    pub plan_json: String,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
    pub snapshot_json: Option<String>,
    pub steps: Vec<StepRecord>,
}

pub struct NewProject<'a> {
    pub id: &'a str,
    pub local_path: &'a str,
    pub github_host: Option<&'a str>,
    pub owner: Option<&'a str>,
    pub repo: Option<&'a str>,
    pub remote_name: Option<&'a str>,
    pub now: &'a str,
}

pub trait OperationStore {
    fn upsert_project(&self, project: NewProject<'_>) -> Result<ProjectRecord, AppError>;
    fn get_project_by_path(&self, path: &Path) -> Result<Option<ProjectRecord>, AppError>;
    fn create_operation(
        &self,
        project_id: &str,
        id: &str,
        kind: &str,
        status: &str,
        plan: &OperationPlan,
        snapshot: &RepositorySnapshot,
        started_at: &str,
    ) -> Result<OperationRecord, AppError>;
    fn update_operation(
        &self,
        id: &str,
        status: &str,
        completed_at: Option<&str>,
    ) -> Result<(), AppError>;
    fn append_step(
        &self,
        operation_id: &str,
        id: &str,
        sequence: i32,
        kind: &str,
        status: StepStatus,
        detail_json: Option<&str>,
        now: &str,
    ) -> Result<StepRecord, AppError>;
    fn update_step(
        &self,
        id: &str,
        status: StepStatus,
        output_text: Option<&str>,
        completed_at: Option<&str>,
    ) -> Result<(), AppError>;
    fn list_operations(
        &self,
        project_id: &str,
        limit: u32,
    ) -> Result<Vec<OperationRecord>, AppError>;
}
```

`clock.rs`:

```rust
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

use crate::ports::Clock;

pub struct SystemClock;

impl Clock for SystemClock {
    fn now_rfc3339(&self) -> String {
        OffsetDateTime::now_utc()
            .format(&Rfc3339)
            .expect("rfc3339 formatting cannot fail for utc now")
    }
}
```

`ids.rs`:

```rust
use ulid::Ulid;

use crate::ports::IdGenerator;

pub struct UlidGenerator;

impl IdGenerator for UlidGenerator {
    fn next(&self) -> Ulid {
        Ulid::new()
    }
}
```

`redact.rs`:

```rust
const MAX_OUTPUT_CHARS: usize = 64 * 1024;

pub fn redact(text: &str) -> String {
    let mut out = redact_basic_auth(text);
    out = redact_token_prefix(&out, "ghp_");
    out = redact_token_prefix(&out, "gho_");
    out = redact_token_prefix(&out, "ghu_");
    out = redact_token_prefix(&out, "ghs_");
    out = redact_token_prefix(&out, "github_pat_");
    out = redact_bearer(&out);
    out
}

pub fn bound_output(text: &str) -> String {
    let redacted = redact(text);
    if redacted.chars().count() <= MAX_OUTPUT_CHARS {
        return redacted;
    }
    let truncated: String = redacted.chars().take(MAX_OUTPUT_CHARS).collect();
    format!("{truncated}\n...[truncated]")
}

fn redact_basic_auth(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut last = 0;
    let mut search_from = 0;
    while let Some(rel) = text[search_from..].find("://") {
        let cred_start = search_from + rel + 3;
        out.push_str(&text[last..cred_start]);
        if let Some(rel_at) = text[cred_start..].find('@') {
            let creds = &text[cred_start..cred_start + rel_at];
            if creds.contains(':') && !creds.contains('/') {
                out.push_str("[redacted]");
                last = cred_start + rel_at;
                search_from = last;
                continue;
            }
        }
        last = cred_start;
        search_from = cred_start;
    }
    out.push_str(&text[last..]);
    out
}

fn redact_token_prefix(text: &str, prefix: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(idx) = rest.find(prefix) {
        out.push_str(&rest[..idx]);
        out.push_str("[redacted]");
        rest = &rest[idx + prefix.len()..];
        let skip = rest
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
            .count();
        rest = &rest[rest.char_indices().nth(skip).map(|(i, _)| i).unwrap_or(rest.len())..];
    }
    out.push_str(rest);
    out
}

fn redact_bearer(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(idx) = rest.to_ascii_lowercase().find("bearer ") {
        out.push_str(&rest[..idx]);
        out.push_str("Bearer [redacted]");
        rest = &rest[idx + 7..];
        let skip = rest
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '-' || *c == '.')
            .count();
        rest = &rest[rest.char_indices().nth(skip).map(|(i, _)| i).unwrap_or(rest.len())..];
    }
    out.push_str(rest);
    out
}
```

`remote.rs`:

```rust
use crate::error::AppError;
use workbench_domain::repository::Remote;

pub fn resolve_remote(
    remotes: &[Remote],
    mapped_remote: Option<&str>,
    flag: Option<&str>,
) -> Result<String, AppError> {
    if remotes.is_empty() {
        return Err(AppError::RepositoryNotMapped);
    }
    if let Some(name) = flag {
        return require_existing(remotes, name);
    }
    if let Some(name) = mapped_remote {
        return require_existing(remotes, name);
    }
    if remotes.len() == 1 {
        return Ok(remotes[0].name.clone());
    }
    Err(AppError::RemoteNotResolved {
        candidates: remotes.iter().map(|r| r.name.clone()).collect(),
    })
}

fn require_existing(remotes: &[Remote], name: &str) -> Result<String, AppError> {
    if remotes.iter().any(|r| r.name == name) {
        Ok(name.to_string())
    } else {
        Err(AppError::RemoteNotResolved {
            candidates: remotes.iter().map(|r| r.name.clone()).collect(),
        })
    }
}
```

`policy_source.rs`:

```rust
use std::path::Path;

use crate::error::AppError;
use crate::ports::PolicySource;
use workbench_domain::policy::{github_flow_defaults, parse_policy_yaml, PolicyConfig};

pub struct FilePolicySource;

impl PolicySource for FilePolicySource {
    fn read_yaml(&self, repo_root: &Path) -> Result<Option<String>, AppError> {
        let path = repo_root.join(".github-workbench.yml");
        match std::fs::read_to_string(&path) {
            Ok(body) => Ok(Some(body)),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(err) => Err(AppError::Io {
                path: path.display().to_string(),
                detail: err.to_string(),
            }),
        }
    }
}

pub fn load_policy<P: PolicySource>(
    source: &P,
    repo_root: &Path,
) -> Result<(PolicyConfig, &'static str), AppError> {
    match source.read_yaml(repo_root)? {
        None => Ok((github_flow_defaults(), "defaults")),
        Some(yaml) => {
            let cfg = parse_policy_yaml(&yaml)?;
            Ok((cfg, "file"))
        }
    }
}
```

`fakes.rs`:

```rust
use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use ulid::Ulid;

use crate::error::AppError;
use crate::ports::{
    Clock, CommandOutput, GitClient, IdGenerator, NewProject, OperationRecord, OperationStore,
    PolicySource, ProjectRecord, StepRecord,
};
use workbench_domain::operations::plan::{GitCommand, OperationPlan, StepStatus};
use workbench_domain::repository::{BranchState, Remote, RepositorySnapshot};

pub struct FakeClock(pub String);
impl Clock for FakeClock {
    fn now_rfc3339(&self) -> String {
        self.0.clone()
    }
}

pub struct FakeIds {
    pub next: Mutex<u64>,
}
impl FakeIds {
    pub fn new() -> Self {
        Self {
            next: Mutex::new(1),
        }
    }
}
impl IdGenerator for FakeIds {
    fn next(&self) -> Ulid {
        let mut n = self.next.lock().unwrap();
        let value = *n;
        *n += 1;
        Ulid::from_parts(value, value)
    }
}

pub struct FakePolicy {
    pub yaml: Option<String>,
}
impl PolicySource for FakePolicy {
    fn read_yaml(&self, _repo_root: &Path) -> Result<Option<String>, AppError> {
        Ok(self.yaml.clone())
    }
}

pub struct FakeGit {
    pub toplevel: PathBuf,
    pub snapshot: RefCell<RepositorySnapshot>,
    pub branch: RefCell<BranchState>,
    pub executed: RefCell<Vec<GitCommand>>,
    pub fail_kind: RefCell<Option<String>>,
}

impl FakeGit {
    fn maybe_fail(&self, kind: &str) -> Result<CommandOutput, AppError> {
        if self.fail_kind.borrow().as_deref() == Some(kind) {
            return Err(AppError::GitFailed {
                program: "git".into(),
                args_summary: kind.into(),
                status: 1,
                stderr_redacted: "injected failure".into(),
            });
        }
        Ok(CommandOutput {
            exit_code: 0,
            stdout: String::new(),
            stderr: String::new(),
        })
    }
}

impl GitClient for FakeGit {
    fn resolve_toplevel(&self, _path: &Path) -> Result<PathBuf, AppError> {
        Ok(self.toplevel.clone())
    }

    fn snapshot(&self, _repo_root: &Path) -> Result<RepositorySnapshot, AppError> {
        Ok(self.snapshot.borrow().clone())
    }

    fn branch_state(
        &self,
        _repo_root: &Path,
        _comparison_ref: &str,
    ) -> Result<BranchState, AppError> {
        Ok(self.branch.borrow().clone())
    }

    fn list_remotes(&self, _repo_root: &Path) -> Result<Vec<Remote>, AppError> {
        Ok(self.snapshot.borrow().remotes.clone())
    }

    fn fetch(&self, _repo_root: &Path, remote: &str) -> Result<CommandOutput, AppError> {
        self.executed.borrow_mut().push(GitCommand::Fetch {
            remote: remote.into(),
        });
        self.maybe_fail("fetch")
    }

    fn create_branch(
        &self,
        _repo_root: &Path,
        name: &str,
        start_point: &str,
    ) -> Result<CommandOutput, AppError> {
        self.executed.borrow_mut().push(GitCommand::CreateBranch {
            name: name.into(),
            start_point: start_point.into(),
        });
        let out = self.maybe_fail("create-branch")?;
        self.snapshot.borrow_mut().branch = Some(name.into());
        self.branch.borrow_mut().name = name.into();
        Ok(out)
    }

    fn push_ref(
        &self,
        _repo_root: &Path,
        remote: &str,
        local_ref: &str,
        remote_ref: &str,
        set_upstream: bool,
    ) -> Result<CommandOutput, AppError> {
        self.executed.borrow_mut().push(GitCommand::PushRef {
            remote: remote.into(),
            local_ref: local_ref.into(),
            remote_ref: remote_ref.into(),
            set_upstream,
        });
        self.maybe_fail("push-ref")
    }
}

pub struct FakeStore {
    pub projects: Mutex<Vec<ProjectRecord>>,
    pub operations: Mutex<Vec<OperationRecord>>,
}

impl FakeStore {
    pub fn new() -> Self {
        Self {
            projects: Mutex::new(Vec::new()),
            operations: Mutex::new(Vec::new()),
        }
    }
}

impl OperationStore for FakeStore {
    fn upsert_project(&self, project: NewProject<'_>) -> Result<ProjectRecord, AppError> {
        let mut projects = self.projects.lock().unwrap();
        if let Some(existing) = projects
            .iter_mut()
            .find(|p| p.local_path == project.local_path)
        {
            existing.github_host = project.github_host.map(str::to_string);
            existing.owner = project.owner.map(str::to_string);
            existing.repo = project.repo.map(str::to_string);
            existing.remote_name = project.remote_name.map(str::to_string);
            existing.updated_at = project.now.to_string();
            return Ok(existing.clone());
        }
        let record = ProjectRecord {
            id: project.id.to_string(),
            local_path: project.local_path.to_string(),
            github_host: project.github_host.map(str::to_string),
            owner: project.owner.map(str::to_string),
            repo: project.repo.map(str::to_string),
            remote_name: project.remote_name.map(str::to_string),
            created_at: project.now.to_string(),
            updated_at: project.now.to_string(),
        };
        projects.push(record.clone());
        Ok(record)
    }

    fn get_project_by_path(&self, path: &Path) -> Result<Option<ProjectRecord>, AppError> {
        let key = path.to_string_lossy().into_owned();
        Ok(self
            .projects
            .lock()
            .unwrap()
            .iter()
            .find(|p| p.local_path == key)
            .cloned())
    }

    fn create_operation(
        &self,
        project_id: &str,
        id: &str,
        kind: &str,
        status: &str,
        plan: &OperationPlan,
        snapshot: &RepositorySnapshot,
        started_at: &str,
    ) -> Result<OperationRecord, AppError> {
        let record = OperationRecord {
            id: id.into(),
            project_id: project_id.into(),
            kind: kind.into(),
            status: status.into(),
            plan_json: serde_json::to_string(plan).unwrap(),
            started_at: Some(started_at.into()),
            completed_at: None,
            snapshot_json: Some(serde_json::to_string(snapshot).unwrap()),
            steps: vec![],
        };
        self.operations.lock().unwrap().push(record.clone());
        Ok(record)
    }

    fn update_operation(
        &self,
        id: &str,
        status: &str,
        completed_at: Option<&str>,
    ) -> Result<(), AppError> {
        let mut ops = self.operations.lock().unwrap();
        let op = ops.iter_mut().find(|o| o.id == id).unwrap();
        op.status = status.into();
        op.completed_at = completed_at.map(str::to_string);
        Ok(())
    }

    fn append_step(
        &self,
        operation_id: &str,
        id: &str,
        sequence: i32,
        kind: &str,
        status: StepStatus,
        detail_json: Option<&str>,
        _now: &str,
    ) -> Result<StepRecord, AppError> {
        let step = StepRecord {
            id: id.into(),
            operation_id: operation_id.into(),
            sequence,
            kind: kind.into(),
            status,
            detail_json: detail_json.map(str::to_string),
            output_text: None,
        };
        let mut ops = self.operations.lock().unwrap();
        let op = ops.iter_mut().find(|o| o.id == operation_id).unwrap();
        op.steps.push(step.clone());
        Ok(step)
    }

    fn update_step(
        &self,
        id: &str,
        status: StepStatus,
        output_text: Option<&str>,
        _completed_at: Option<&str>,
    ) -> Result<(), AppError> {
        let mut ops = self.operations.lock().unwrap();
        for op in ops.iter_mut() {
            if let Some(step) = op.steps.iter_mut().find(|s| s.id == id) {
                step.status = status;
                step.output_text = output_text.map(str::to_string);
                return Ok(());
            }
        }
        Err(AppError::Storage {
            detail: format!("missing step {id}"),
        })
    }

    fn list_operations(
        &self,
        project_id: &str,
        limit: u32,
    ) -> Result<Vec<OperationRecord>, AppError> {
        let mut rows: Vec<_> = self
            .operations
            .lock()
            .unwrap()
            .iter()
            .filter(|o| o.project_id == project_id)
            .cloned()
            .collect();
        rows.reverse();
        rows.truncate(limit as usize);
        Ok(rows)
    }
}
```

`lib.rs`:

```rust
pub mod clock;
pub mod error;
pub mod fakes;
pub mod ids;
pub mod policy_source;
pub mod ports;
pub mod redact;
pub mod remote;

pub use error::AppError;
```

- [ ] **Step 5: Run tests — expect PASS**

Run: `cargo test -p workbench-application`

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add Cargo.toml Cargo.lock crates/workbench-application
git commit -m "feat(application): add ports, AppError, redaction, and fakes"
```

---

### Task 3: Process runner and Git argv (never force)

**Files:**
- Modify: `crates/workbench-git/Cargo.toml`
- Modify: `crates/workbench-git/README.md`
- Modify: `crates/workbench-git/src/lib.rs`
- Create: `crates/workbench-git/src/argv.rs`
- Create: `crates/workbench-git/src/env.rs`
- Create: `crates/workbench-git/src/process.rs`

**Interfaces:**
- Consumes: `CommandSpec`, `CommandOutput`, `AppError`, `GitCommand`
- Produces:
  - `fn command_argv(cmd: &GitCommand) -> Vec<String>` (no `git` program prefix; args only)
  - `fn describe_command(cmd: &GitCommand) -> String`
  - `fn sanitized_env(overrides: &[(String, String)]) -> Vec<(String, String)>`
  - `struct StdProcessRunner;` implementing `ProcessRunner`
  - `fn assert_no_force(args: &[String])` used by argv construction

- [ ] **Step 1: Write failing tests in `argv.rs` and `process.rs`**

```rust
#[test]
fn push_argv_never_contains_force() {
    let cmd = GitCommand::PushRef {
        remote: "github".into(),
        local_ref: "feature/x".into(),
        remote_ref: "feature/x".into(),
        set_upstream: true,
    };
    let args = command_argv(&cmd);
    assert!(args.iter().any(|a| a == "-u"));
    assert_eq!(args.last().as_deref(), Some("feature/x:feature/x"));
    assert!(!args.iter().any(|a| a == "--force"
        || a == "--force-with-lease"
        || a.starts_with("--force=")
        || a.starts_with("--force-with-lease=")));
}

#[test]
fn create_branch_uses_checkout_b_and_dashdash() {
    let args = command_argv(&GitCommand::CreateBranch {
        name: "feature/42-add-resumable-uploads".into(),
        start_point: "main".into(),
    });
    assert_eq!(
        args,
        vec!["checkout", "-b", "feature/42-add-resumable-uploads", "--", "main"]
    );
}

#[test]
fn fetch_uses_dashdash() {
    assert_eq!(
        command_argv(&GitCommand::Fetch {
            remote: "github".into()
        }),
        vec!["fetch", "--", "github"]
    );
}
```

Process runner test: run `git --version` via `StdProcessRunner` with `sanitized_env(&[])` and cwd of a temp dir. Assert exit 0 and stdout contains `"git version"`. If `git` is missing, the error must be `AppError::GitUnavailable`.

- [ ] **Step 2: Run — expect FAIL**

Run: `cargo test -p workbench-git`

- [ ] **Step 3: Implement**

`argv.rs`:

```rust
use workbench_domain::operations::plan::GitCommand;

pub fn command_argv(cmd: &GitCommand) -> Vec<String> {
    let args = match cmd {
        GitCommand::Fetch { remote } => vec!["fetch".into(), "--".into(), remote.clone()],
        GitCommand::CreateBranch { name, start_point } => vec![
            "checkout".into(),
            "-b".into(),
            name.clone(),
            "--".into(),
            start_point.clone(),
        ],
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
            args
        }
    };
    assert_no_force(&args);
    args
}

pub fn describe_command(cmd: &GitCommand) -> String {
    let mut parts = vec!["git".to_string()];
    parts.extend(command_argv(cmd));
    parts.join(" ")
}

pub fn assert_no_force(args: &[String]) {
    let forbidden = args.iter().any(|a| {
        a == "--force"
            || a == "--force-with-lease"
            || a.starts_with("--force=")
            || a.starts_with("--force-with-lease=")
    });
    assert!(
        !forbidden,
        "force push arguments are forbidden in Phase 2: {args:?}"
    );
}
```

`env.rs`: copy these keys from the current process if present: `PATH`, `HOME`, `USER`, `USERNAME`, `USERPROFILE`, `LANG`, `LC_ALL`, `LC_CTYPE`, `TERM`, `TMPDIR`, `TMP`, `TEMP`, `SSH_AUTH_SOCK`, `SSH_AGENT_PID`, `GIT_ASKPASS`, `GIT_SSH`, `GIT_EXEC_PATH`, `GIT_CONFIG_GLOBAL`, `GIT_CONFIG_SYSTEM`, `GIT_CONFIG_NOSYSTEM`, `GIT_AUTHOR_NAME`, `GIT_AUTHOR_EMAIL`, `GIT_COMMITTER_NAME`, `GIT_COMMITTER_EMAIL`. Always set `GIT_TERMINAL_PROMPT=0`. Apply `overrides` last so test config wins. Do not pass `GWW_GIT_PROGRAM` into the child environment; it only selects `CommandSpec.program`.

`process.rs`: `StdProcessRunner::run` uses `std::process::Command::new(&spec.program).args(&spec.args).current_dir(&spec.cwd).env_clear().envs(spec.env.clone())`, captures stdout/stderr with `String::from_utf8_lossy`, maps `ErrorKind::NotFound` to `GitUnavailable`. Phase 2 `run` is blocking; cancellation is not implemented.

`Cargo.toml` for git: depend on `workbench-application`, `workbench-domain`, `thiserror`; dev-dep `tempfile`, `pretty_assertions`.

README: replace stub with “Process-based Git adapter. Phase 2.”

- [ ] **Step 4: Run — expect PASS**

Run: `cargo test -p workbench-git`

- [ ] **Step 5: Commit**

```bash
git add crates/workbench-git Cargo.lock
git commit -m "feat(git): add argv builders and StdProcessRunner"
```

---

### Task 4: Git parsers and ProcessGitClient integration tests

**Files:**
- Create: `crates/workbench-git/src/parser.rs`
- Create: `crates/workbench-git/src/client.rs`
- Create: `crates/workbench-git/tests/parser.rs`
- Create: `crates/workbench-git/tests/git_integration.rs`
- Modify: `crates/workbench-git/src/lib.rs`

**Interfaces:**
- Consumes: `ProcessRunner`, `GitClient`, `GitCommand`, `RepositorySnapshot`, `BranchState`, `Remote`, `command_argv`, `sanitized_env`
- Produces:
  - `fn parse_porcelain_z(stdout: &str) -> Vec<String>`
  - `fn parse_remotes_verbose(stdout: &str) -> Vec<Remote>` (unique by name, prefer fetch URL)
  - `fn parse_ahead_behind(stdout: &str) -> Result<(u64, u64), AppError>` for `git rev-list --left-right --count A...B` (`ahead\\tbehind`)
  - `struct ProcessGitClient<R> { runner, git_program, extra_env }` implementing `GitClient`
  - `ProcessGitClient::new(runner)` reads `GWW_GIT_PROGRAM` or `"git"`; `with_extra_env(self, Vec<(String,String)>)` overrides sanitized env keys
  - `create_branch` executes `git checkout -b <name> -- <start_point>`. If `git show-ref --verify --quiet refs/heads/<name>` succeeds first, run `git checkout -- <name>` instead and treat as success (idempotent existing branch).
  - `push_ref` uses `command_argv` for `PushRef` (never force).
  - Isolated integration harness with temp `HOME`, `GIT_CONFIG_NOSYSTEM=1`, `GIT_CONFIG_GLOBAL` pointing at a written config (`user.name=Workbench Test`, `user.email=workbench@example.test`, `init.defaultBranch=main`, `commit.gpgsign=false`).

- [ ] **Step 1: Write failing parser tests**

```rust
#[test]
fn porcelain_z_lists_modified_and_untracked() {
    let raw = " M file.txt\0?? hello world.txt\0";
    let paths = parse_porcelain_z(raw);
    assert_eq!(paths, vec!["file.txt", "hello world.txt"]);
}

#[test]
fn remotes_verbose_dedupes_fetch_and_push() {
    let raw = "origin\tgit@github.com:acme/widgets.git (fetch)\norigin\tgit@github.com:acme/widgets.git (push)\ngithub\thttps://github.com/acme/widgets.git (fetch)\n";
    let remotes = parse_remotes_verbose(raw);
    assert_eq!(remotes.len(), 2);
    assert_eq!(remotes[0].name, "origin");
    assert_eq!(remotes[1].name, "github");
}

#[test]
fn ahead_behind_tab_separated() {
    assert_eq!(parse_ahead_behind("2\t3\n").unwrap(), (2, 3));
}
```

`git_integration.rs` helper and test (real Git, isolated config):

```rust
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::TempDir;
use workbench_application::ports::GitClient;
use workbench_git::argv::assert_no_force;
use workbench_git::{ProcessGitClient, StdProcessRunner, sanitized_env};

struct Harness {
    _tmp: TempDir,
    home: PathBuf,
    remote: PathBuf,
    work: PathBuf,
    extra_env: Vec<(String, String)>,
}

fn write_gitconfig(home: &Path) {
    fs::write(
        home.join("gitconfig"),
        "[user]\n    name = Workbench Test\n    email = workbench@example.test\n[init]\n    defaultBranch = main\n[commit]\n    gpgsign = false\n",
    )
    .unwrap();
}

fn git(home: &Path, cwd: &Path, args: &[&str]) {
    let config = home.join("gitconfig");
    let status = Command::new("git")
        .args(args)
        .current_dir(cwd)
        .env_clear()
        .envs(sanitized_env(&[
            ("HOME".into(), home.display().to_string()),
            ("GIT_CONFIG_NOSYSTEM".into(), "1".into()),
            ("GIT_CONFIG_GLOBAL".into(), config.display().to_string()),
        ]))
        .status()
        .expect("git must be installed");
    assert!(status.success(), "git {args:?} failed");
}

fn harness() -> Harness {
    let tmp = TempDir::new().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    write_gitconfig(&home);
    let remote = tmp.path().join("remote.git");
    let work = tmp.path().join("work");
    git(&home, tmp.path(), &["init", "--bare", "-b", "main", remote.to_str().unwrap()]);
    git(&home, tmp.path(), &["clone", remote.to_str().unwrap(), work.to_str().unwrap()]);
    fs::write(work.join("README.md"), "hi\n").unwrap();
    git(&home, &work, &["add", "README.md"]);
    git(&home, &work, &["commit", "-m", "init"]);
    git(&home, &work, &["push", "-u", "origin", "main"]);
    let extra_env = vec![
        ("HOME".into(), home.display().to_string()),
        ("GIT_CONFIG_NOSYSTEM".into(), "1".into()),
        (
            "GIT_CONFIG_GLOBAL".into(),
            home.join("gitconfig").display().to_string(),
        ),
    ];
    Harness {
        _tmp: tmp,
        home,
        remote,
        work,
        extra_env,
    }
}

#[test]
fn create_branch_push_status_and_dirty_space_path() {
    let h = harness();
    let client = ProcessGitClient::new(StdProcessRunner).with_extra_env(h.extra_env.clone());
    let root = client.resolve_toplevel(&h.work).unwrap();
    assert_eq!(root, h.work.canonicalize().unwrap());

    let snap = client.snapshot(&root).unwrap();
    assert!(!snap.detached_head);
    assert_eq!(snap.branch.as_deref(), Some("main"));
    assert!(snap.dirty_paths.is_empty());
    assert_eq!(snap.remotes.len(), 1);
    let remote_name = snap.remotes[0].name.clone();

    client
        .create_branch(&root, "feature/42-add-resumable-uploads", "main")
        .unwrap();
    let snap = client.snapshot(&root).unwrap();
    assert_eq!(
        snap.branch.as_deref(),
        Some("feature/42-add-resumable-uploads")
    );

    fs::write(root.join("note.txt"), "n\n").unwrap();
    git(&h.home, &root, &["add", "note.txt"]);
    git(&h.home, &root, &["commit", "-m", "note"]);
    let before = client.branch_state(&root, "main").unwrap();
    assert!(before.ahead >= 1);

    client
        .push_ref(
            &root,
            &remote_name,
            "feature/42-add-resumable-uploads",
            "feature/42-add-resumable-uploads",
            true,
        )
        .unwrap();
    let after = client.branch_state(&root, "main").unwrap();
    assert_eq!(after.ahead, 0);

    fs::write(root.join("hello world.txt"), "x\n").unwrap();
    let dirty = client.snapshot(&root).unwrap();
    assert!(dirty.dirty_paths.iter().any(|p| p.contains("hello world.txt")));

    assert_no_force(&["push".into(), "-u".into(), "--".into(), remote_name, "feature/42-add-resumable-uploads:feature/42-add-resumable-uploads".into()]);
    let _ = h.remote;
}
```

If `git` is missing, `Command::new("git")` fails the test with `git must be installed` (CI images include Git).

- [ ] **Step 2: Run — expect FAIL**

Run: `cargo test -p workbench-git`

- [ ] **Step 3: Implement parser + client**

`ProcessGitClient` methods run `git` through `ProcessRunner` with argv only. Typical commands:

- toplevel: `rev-parse --show-toplevel`
- HEAD oid: `rev-parse HEAD`
- detached?: `symbolic-ref -q HEAD` (nonzero => detached)
- branch name: `rev-parse --abbrev-ref HEAD`
- porcelain: `status --porcelain=v1 -z`
- remotes: `remote -v`
- upstream: `rev-parse --abbrev-ref --symbolic-full-name @{u}` (nonzero => None)
- ahead/behind with upstream: `rev-list --left-right --count HEAD...@{u}`
- ahead/behind without upstream: `rev-list --left-right --count HEAD...{comparison_ref}` then `(ahead, behind)`
- fetch / create / push: `command_argv`
- existing branch: `show-ref --verify --quiet refs/heads/<name>`
- checkout existing: `checkout -- <name>`
- start_point resolution in `create_branch`: run `rev-parse --verify --quiet -- <start_point>`. If that fails, call `list_remotes` and try `rev-parse --verify --quiet -- <remote.name>/<start_point>` for each remote in listed order. Use the first candidate that exists. If none exist, keep the original `start_point` and let `checkout -b` fail as `GitFailed`. Never invent the name `origin`.

Nonzero git exit (except the expected “no upstream” / “not detached” probes) maps to `GitFailed` with `args_summary` joined by spaces and `stderr_redacted: bound_output(&stderr)`.

`NotAGitRepository` when `rev-parse --show-toplevel` fails with “not a git repository”.

`lib.rs` modules: `argv`, `env`, `process`, `parser`, `client`; re-export `ProcessGitClient`, `StdProcessRunner`, `describe_command`, `sanitized_env`.

- [ ] **Step 4: Run — expect PASS**

Run: `cargo test -p workbench-git`

Expected: PASS, including integration tests.

- [ ] **Step 5: Commit**

```bash
git add crates/workbench-git
git commit -m "feat(git): parse status/remotes and run real git via ProcessGitClient"
```

---

### Task 5: SQLite store and migrations

**Files:**
- Modify: `crates/workbench-storage/Cargo.toml`
- Modify: `crates/workbench-storage/README.md`
- Modify: `crates/workbench-storage/src/lib.rs`
- Create: `crates/workbench-storage/src/sqlite.rs`
- Create: `crates/workbench-storage/src/migrations.rs`
- Create: `crates/workbench-storage/src/migrations/001_initial.sql`
- Create: `crates/workbench-storage/tests/sqlite_store.rs`

**Interfaces:**
- Consumes: `OperationStore`, `NewProject`, `OperationPlan`, `RepositorySnapshot`, `StepStatus`, `AppError`
- Produces: `SqliteStore::open(path) -> Result<SqliteStore, AppError>` applying migrations; implements `OperationStore`

- [ ] **Step 1: Write failing round-trip test**

```rust
#[test]
fn migrations_and_operation_round_trip() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("workbench.db");
    let store = SqliteStore::open(&db).unwrap();
    // open again to prove migrations are idempotent
    let store = SqliteStore::open(&db).unwrap();

    let project = store
        .upsert_project(NewProject {
            id: "01ARZ3NDEKTSV4RRFFQ69G5FAV",
            local_path: "/tmp/repo",
            github_host: Some("github.com"),
            owner: Some("acme"),
            repo: Some("widgets"),
            remote_name: Some("github"),
            now: "2026-08-24T00:00:00Z",
        })
        .unwrap();
    assert_eq!(project.remote_name.as_deref(), Some("github"));

    let plan = OperationPlan {
        id: ulid::Ulid::nil(),
        kind: "push".into(),
        risk: RiskClass::Low,
        summary: "test".into(),
        rationale: vec![],
        commands: vec![],
        preconditions: vec![],
        findings: vec![],
    };
    let snapshot = RepositorySnapshot {
        root: "/tmp/repo".into(),
        branch: Some("feature/x".into()),
        detached_head: false,
        head_oid: Some("abc".into()),
        dirty_paths: vec![],
        remotes: vec![],
        selected_remote: Some("github".into()),
        upstream: None,
    };
    let op = store
        .create_operation(
            &project.id,
            "01ARZ3NDEKTSV4RRFFQ69G5FAW",
            "push",
            "running",
            &plan,
            &snapshot,
            "2026-08-24T00:00:01Z",
        )
        .unwrap();
    let step = store
        .append_step(
            &op.id,
            "01ARZ3NDEKTSV4RRFFQ69G5FAX",
            1,
            "push-ref",
            StepStatus::Pending,
            None,
            "2026-08-24T00:00:01Z",
        )
        .unwrap();
    store
        .update_step(
            &step.id,
            StepStatus::Succeeded,
            Some("pushed"),
            Some("2026-08-24T00:00:02Z"),
        )
        .unwrap();
    store
        .update_operation(&op.id, "succeeded", Some("2026-08-24T00:00:02Z"))
        .unwrap();

    let listed = store.list_operations(&project.id, 10).unwrap();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].status, "succeeded");
    assert_eq!(listed[0].steps.len(), 1);
    assert_eq!(listed[0].steps[0].status, StepStatus::Succeeded);
    assert_eq!(listed[0].steps[0].output_text.as_deref(), Some("pushed"));
}
```

- [ ] **Step 2: Run — expect FAIL**

Run: `cargo test -p workbench-storage`

- [ ] **Step 3: Implement schema and store**

`001_initial.sql`:

```sql
CREATE TABLE projects (
    id TEXT PRIMARY KEY,
    local_path TEXT NOT NULL UNIQUE,
    github_host TEXT,
    owner TEXT,
    repo TEXT,
    remote_name TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE operations (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES projects(id),
    kind TEXT NOT NULL,
    status TEXT NOT NULL,
    plan_json TEXT NOT NULL,
    snapshot_json TEXT,
    started_at TEXT,
    completed_at TEXT
);

CREATE TABLE operation_steps (
    id TEXT PRIMARY KEY,
    operation_id TEXT NOT NULL REFERENCES operations(id),
    sequence INTEGER NOT NULL,
    kind TEXT NOT NULL,
    status TEXT NOT NULL,
    detail_json TEXT,
    output_text TEXT,
    started_at TEXT,
    completed_at TEXT
);

CREATE INDEX idx_operations_project_started ON operations(project_id, started_at DESC);
CREATE INDEX idx_steps_operation_sequence ON operation_steps(operation_id, sequence);
```

`migrations.rs`: table `schema_migrations(id TEXT PRIMARY KEY, applied_at TEXT NOT NULL)`; apply each embedded SQL file once inside a transaction.

`SqliteStore` holds `Mutex<Connection>`. Map rusqlite errors to `AppError::Storage`. Serialize `plan` / `snapshot` with `serde_json`. Persist `StepStatus` via serde (`pending`, `running`, `succeeded`, `failed`, `skipped`, `compensation-needed`). `upsert_project` updates mapping columns and `updated_at` when `local_path` already exists (keep original `id` and `created_at`). `list_operations` newest first, attach steps ordered by `sequence`.

Do not store tokens or unredacted output. Callers pass already-redacted `output_text`.

- [ ] **Step 4: Run — expect PASS**

Run: `cargo test -p workbench-storage`

- [ ] **Step 5: Commit**

```bash
git add crates/workbench-storage Cargo.lock
git commit -m "feat(storage): add SQLite projects and operation journal"
```

---

### Task 6: Open repository and status use cases

**Files:**
- Create: `crates/workbench-application/src/use_cases/mod.rs`
- Create: `crates/workbench-application/src/use_cases/open.rs`
- Create: `crates/workbench-application/src/use_cases/status.rs`
- Create: `crates/workbench-application/src/recommend.rs`
- Create: `crates/workbench-application/tests/open_and_status.rs`
- Modify: `crates/workbench-application/src/lib.rs`

**Interfaces:**
- Consumes: ports, `load_policy`, `resolve_remote`, `parse_github_remote`, `evaluate_current_branch_policy`
- Produces:
  - `OpenOutcome { snapshot, project, policy, policy_source: &'static str }`
  - `fn open_repository(git, store, policy, clock, ids, path, remote_flag) -> Result<OpenOutcome, AppError>`
  - `StatusOutcome { snapshot, branch, policy, policy_source, findings, recommended_next_action }`
  - `fn repository_status(git, policy, path, mapped_remote, remote_flag) -> Result<StatusOutcome, AppError>`
  - `fn recommend_next_action(policy, snapshot, branch) -> String`

- [ ] **Step 1: Write failing application tests using fakes**

`crates/workbench-application/tests/open_and_status.rs` (include all of these tests):

```rust
use std::cell::RefCell;
use std::path::PathBuf;
use workbench_application::fakes::{FakeClock, FakeGit, FakeIds, FakePolicy, FakeStore};
use workbench_application::ports::OperationStore;
use workbench_application::use_cases::open::open_repository;
use workbench_application::use_cases::status::repository_status;
use workbench_application::AppError;
use workbench_domain::repository::{BranchState, Remote, RepositorySnapshot};

fn github_remote() -> Remote {
    Remote {
        name: "github".into(),
        url: "git@github.com:acme/widgets.git".into(),
    }
}

fn snap(branch: &str, dirty: Vec<String>, remotes: Vec<Remote>, detached: bool) -> RepositorySnapshot {
    RepositorySnapshot {
        root: "/tmp/repo".into(),
        branch: if detached { None } else { Some(branch.into()) },
        detached_head: detached,
        head_oid: Some("abc".into()),
        dirty_paths: dirty,
        remotes,
        selected_remote: None,
        upstream: None,
    }
}

fn branch(name: &str, ahead: u64, dirty: Vec<String>) -> BranchState {
    BranchState {
        name: name.into(),
        head_oid: Some("abc".into()),
        upstream: None,
        base_branch: Some("main".into()),
        ahead,
        behind: 0,
        dirty_paths: dirty,
        is_protected: name == "main",
    }
}

fn git(snapshot: RepositorySnapshot, branch: BranchState) -> FakeGit {
    FakeGit {
        toplevel: PathBuf::from("/tmp/repo"),
        snapshot: RefCell::new(snapshot),
        branch: RefCell::new(branch),
        executed: RefCell::new(vec![]),
        fail_kind: RefCell::new(None),
    }
}

#[test]
fn open_records_project_from_sole_remote() {
    let git = git(snap("main", vec![], vec![github_remote()], false), branch("main", 0, vec![]));
    let store = FakeStore::new();
    let out = open_repository(
        &git,
        &store,
        &FakePolicy { yaml: None },
        &FakeClock("2026-08-24T00:00:00Z".into()),
        &FakeIds::new(),
        PathBuf::from("/tmp/repo").as_path(),
        None,
    )
    .unwrap();
    assert_eq!(out.policy_source, "defaults");
    assert_eq!(out.project.remote_name.as_deref(), Some("github"));
    assert_eq!(out.project.owner.as_deref(), Some("acme"));
    assert_eq!(out.project.repo.as_deref(), Some("widgets"));
    assert_eq!(store.projects.lock().unwrap().len(), 1);
}

#[test]
fn invalid_policy_does_not_write_sqlite() {
    let git = git(snap("main", vec![], vec![github_remote()], false), branch("main", 0, vec![]));
    let store = FakeStore::new();
    let err = open_repository(
        &git,
        &store,
        &FakePolicy {
            yaml: Some("schema-version: 1\nstrategy:\n  preset: github-flow\n  default-branch: main\ntypo-field: true\n".into()),
        },
        &FakeClock("2026-08-24T00:00:00Z".into()),
        &FakeIds::new(),
        PathBuf::from("/tmp/repo").as_path(),
        None,
    )
    .unwrap_err();
    assert!(matches!(err, AppError::Domain(_)));
    assert!(store.projects.lock().unwrap().is_empty());
}

#[test]
fn two_remotes_without_flag_do_not_write() {
    let remotes = vec![
        github_remote(),
        Remote { name: "other".into(), url: "git@github.com:acme/other.git".into() },
    ];
    let git = git(snap("main", vec![], remotes, false), branch("main", 0, vec![]));
    let store = FakeStore::new();
    let err = open_repository(&git, &store, &FakePolicy { yaml: None }, &FakeClock("2026-08-24T00:00:00Z".into()), &FakeIds::new(), PathBuf::from("/tmp/repo").as_path(), None).unwrap_err();
    assert!(matches!(err, AppError::RemoteNotResolved { .. }));
    assert!(store.projects.lock().unwrap().is_empty());
}

#[test]
fn status_recommends_start_issue_on_clean_main() {
    let git = git(snap("main", vec![], vec![github_remote()], false), branch("main", 0, vec![]));
    let store = FakeStore::new();
    let out = repository_status(&git, &FakePolicy { yaml: None }, PathBuf::from("/tmp/repo").as_path(), None, None).unwrap();
    assert!(out.recommended_next_action.contains("gww issue start"));
    assert!(store.projects.lock().unwrap().is_empty());
}

#[test]
fn status_recommends_push_when_ahead() {
    let git = git(
        snap("feature/42-add-resumable-uploads", vec![], vec![github_remote()], false),
        branch("feature/42-add-resumable-uploads", 2, vec![]),
    );
    let out = repository_status(&git, &FakePolicy { yaml: None }, PathBuf::from("/tmp/repo").as_path(), None, None).unwrap();
    assert!(out.recommended_next_action.contains("gww push --plan"));
}

#[test]
fn status_recommends_commit_when_dirty_feature_branch() {
    let git = git(
        snap("feature/42-add-resumable-uploads", vec!["a.txt".into()], vec![github_remote()], false),
        branch("feature/42-add-resumable-uploads", 0, vec!["a.txt".into()]),
    );
    let out = repository_status(&git, &FakePolicy { yaml: None }, PathBuf::from("/tmp/repo").as_path(), None, None).unwrap();
    assert!(out.recommended_next_action.contains("Commit your changes"));
}

#[test]
fn status_recommends_checkout_when_detached() {
    let git = git(snap("HEAD", vec![], vec![github_remote()], true), branch("HEAD", 0, vec![]));
    let out = repository_status(&git, &FakePolicy { yaml: None }, PathBuf::from("/tmp/repo").as_path(), None, None).unwrap();
    assert!(out.recommended_next_action.contains("Check out a branch"));
}
```

- [ ] **Step 2: Run — expect FAIL**

Run: `cargo test -p workbench-application --test open_and_status`

- [ ] **Step 3: Implement**

`open_repository` algorithm:

1. `git.resolve_toplevel(path)`. If Git reports not a repository, return `NotAGitRepository`.
2. `let mut snapshot = git.snapshot(root)?`.
3. `load_policy(policy, root)?`. On `InvalidPolicy`, return that error. Do not call `upsert_project`.
4. `let remote = resolve_remote(&snapshot.remotes, None, remote_flag)?` — empty remotes → `RepositoryNotMapped`; multiple remotes without `--remote` → `RemoteNotResolved`. Either error returns before `upsert_project`.
5. `snapshot.selected_remote = Some(remote.clone())`.
6. Look up the selected remote URL on `snapshot.remotes` and call `parse_github_remote`. If it returns `Some(identity)`, store `host`/`owner`/`name`; if `None`, store those columns as `None` and still succeed.
7. `store.upsert_project(...)`.
8. Return `OpenOutcome`.

`repository_status` algorithm:

1. Resolve toplevel + snapshot.
2. `load_policy` (invalid → error). Status never writes SQLite.
3. `branch_state(root, snapshot.upstream.as_deref().unwrap_or(&policy.strategy.default_branch))`.
4. `resolve_remote(&snapshot.remotes, mapped_remote, remote_flag)`: on `Ok(name)` set `snapshot.selected_remote = Some(name)`; on `Err(RemoteNotResolved | RepositoryNotMapped)` set `selected_remote = None` and continue (status remains observational).
5. `findings = evaluate_current_branch_policy(&policy, branch.name)`.
6. `recommended_next_action = recommend_next_action(&policy, &snapshot, &branch)`.

`recommend.rs`:

```rust
use workbench_domain::policy::PolicyConfig;
use workbench_domain::repository::{BranchState, RepositorySnapshot};

pub fn recommend_next_action(
    policy: &PolicyConfig,
    snapshot: &RepositorySnapshot,
    branch: &BranchState,
) -> String {
    if snapshot.detached_head {
        return "Check out a branch before starting work (for example git checkout main).".into();
    }
    let on_default = branch.name == policy.strategy.default_branch;
    if !snapshot.dirty_paths.is_empty() {
        if on_default {
            return "Commit or stash local changes, then start an issue branch with gww issue start <n> --title <text>.".into();
        }
        return "Commit your changes with Git, then run gww push --plan.".into();
    }
    if on_default {
        return "Start a policy-compliant feature branch: gww issue start <n> --title <text>.".into();
    }
    if branch.ahead > 0 {
        return "Preview and push this branch: gww push --plan.".into();
    }
    "Nothing to push. Commit new work, or create a draft pull request (Phase 3+).".into()
}
```

Export `pub mod recommend;` and `pub mod use_cases;` from `lib.rs`. `use_cases/mod.rs`:

```rust
pub mod open;
pub mod status;
```

- [ ] **Step 4: Run — expect PASS**

Run: `cargo test -p workbench-application --test open_and_status`

- [ ] **Step 5: Commit**

```bash
git add crates/workbench-application
git commit -m "feat(application): open repository and status use cases"
```

---

### Task 7: Plan executor and issue-start use case

**Files:**
- Create: `crates/workbench-application/src/executor.rs`
- Create: `crates/workbench-application/src/use_cases/start_issue.rs`
- Create: `crates/workbench-application/tests/start_issue.rs`
- Modify: `crates/workbench-application/src/use_cases/mod.rs`
- Modify: `crates/workbench-application/src/lib.rs`

**Interfaces:**
- Consumes: `plan_create_branch_from_issue`, `GitCommand`, `OperationStore`, `GitClient`, `RiskClass::High` forbidden
- Produces:
  - `ExecuteOutcome { operation_id: String, status: String, changed: Vec<String> }`
  - `fn execute_plan(git, store, clock, ids, project_id, snapshot, plan) -> Result<ExecuteOutcome, AppError>`
  - `fn plan_start_issue(git, store, policy, path, issue, title, remote_flag) -> Result<(OperationPlan, RepositorySnapshot, PolicyConfig), AppError>`
  - `fn execute_start_issue(git, store, policy, clock, ids, path, issue, title, remote_flag) -> Result<ExecuteOutcome, AppError>`

`plan_start_issue` reads `store.get_project_by_path` for a mapped remote and does not upsert. `execute_start_issue` calls `plan_start_issue`, then upserts the project, then `execute_plan`.

- [ ] **Step 1: Write failing tests**

1. Fake on `main`, `plan_start_issue(..., 42, "Add resumable uploads", None)` summary contains `feature/42-add-resumable-uploads`, commands are `[CreateBranch]` (already on start_point), no store mutation yet.
2. `--remote` / mapped remote `github` while current branch is not `main` → first command `Fetch { remote: "github" }`.
3. `execute_plan` journals pending → running → succeeded for each command; operation status `succeeded`; `FakeGit.executed` matches plan commands.
4. `FakeGit.fail_kind = Some("create-branch")` → step `failed`, later steps `skipped` if any, operation `failed`, `AppError::OperationFailed` with `changed` listing succeeded steps only, `retry_safe` false if a create-branch/push-ref started, true if only fetch failed.
5. `plan.risk == High` → `AppError::Usage` and no git calls.
6. Unknown high-risk commands cannot be constructed (compile-time allowlist is the `GitCommand` enum). Still match exhaustively in the executor.

- [ ] **Step 2: Run — expect FAIL**

Run: `cargo test -p workbench-application --test start_issue`

- [ ] **Step 3: Implement executor + start_issue**

Executor algorithm:

1. If `plan.risk == RiskClass::High` → `Usage { message: "high-risk operations are not allowed in Phase 2" }`.
2. If `plan.commands.is_empty()` → do **not** create an operation; return `ExecuteOutcome { operation_id: "".into(), status: "noop".into(), changed: vec![] }` (used by nothing-to-push).
3. `create_operation` status `running` with snapshot JSON.
4. For each command at index `sequence` (1-based):
   - `append_step` pending, then `update_step` running.
   - `match command { Fetch => git.fetch, CreateBranch => git.create_branch, PushRef => git.push_ref }` only.
   - On success: `update_step` succeeded with `bound_output` of stdout+stderr; push description onto `changed`.
   - On failure: `update_step` failed with redacted stderr; mark remaining commands `skipped` (append if not yet created); `update_operation` failed; return `OperationFailed` with what changed / what did not / retry_safe / remediation.
5. `update_operation` succeeded.

`plan_start_issue`:

1. toplevel + snapshot + `load_policy` (invalid policy: return error, no store write).
2. `mapped = store.get_project_by_path(root)?.and_then(|p| p.remote_name)`.
3. `remote = resolve_remote(&snapshot.remotes, mapped.as_deref(), flag)?`.
4. `branch_state(root, policy.branches.feature.start_from)`.
5. `plan_create_branch_from_issue(&policy, issue, title, &branch, &remote)`.
6. Return `(plan, snapshot, policy)` without calling `upsert_project`.

`execute_start_issue`:

1. `(plan, snapshot, policy) = plan_start_issue(...)`.
2. Resolve remote again with the same inputs (or reuse `snapshot.selected_remote` after setting it during plan). Upsert the project (`NewProject` with parsed GitHub identity when `parse_github_remote` succeeds).
3. `execute_plan(...)`.

`use_cases/mod.rs` add `pub mod start_issue;`. `lib.rs` add `pub mod executor;`.

- [ ] **Step 4: Run — expect PASS**

Run: `cargo test -p workbench-application --test start_issue`

- [ ] **Step 5: Commit**

```bash
git add crates/workbench-application
git commit -m "feat(application): execute allowlisted plans and start issue branches"
```

---

### Task 8: Push use cases and ops list

**Files:**
- Create: `crates/workbench-application/src/use_cases/push.rs`
- Create: `crates/workbench-application/src/use_cases/ops.rs`
- Create: `crates/workbench-application/tests/push.rs`
- Modify: `crates/workbench-application/src/use_cases/mod.rs`

**Interfaces:**
- Consumes: `plan_push`, `execute_plan`, `resolve_remote`
- Produces:
  - `fn plan_push_changes(git, store, policy, path, remote_flag) -> Result<(OperationPlan, RepositorySnapshot), AppError>`
  - `fn execute_push(git, store, policy, clock, ids, path, remote_flag) -> Result<ExecuteOutcome, AppError>`
  - `fn list_project_operations(git, store, path, limit) -> Result<Vec<OperationRecord>, AppError>`

- [ ] **Step 1: Write failing tests**

1. Dirty `dirty_paths: ["a.txt"]` → `AppError::DirtyWorkingTree { paths }` for both plan and execute; `FakeGit.executed` empty; no operation row.
2. Clean feature branch `ahead: 2`, remote `github` → plan commands Fetch + PushRef, `set_upstream: true` when `upstream: None`.
3. `ahead: 0` → empty commands, summary contains `Nothing to push`; execute returns noop and no operation row.
4. Default branch `main` with `ahead: 1` → `ProtectedBranchMisuse` via `AppError::Domain`.
5. Two remotes, no mapping/flag → `RemoteNotResolved`.
6. After a successful execute, `list_project_operations` returns the push operation with succeeded steps.
7. `list_project_operations` without a stored project → `RepositoryNotMapped`.

- [ ] **Step 2: Run — expect FAIL**

Run: `cargo test -p workbench-application --test push`

- [ ] **Step 3: Implement**

`plan_push_changes`:

1. toplevel, snapshot, `load_policy`.
2. If `!snapshot.dirty_paths.is_empty()` → `DirtyWorkingTree { paths: snapshot.dirty_paths }`.
3. If `snapshot.detached_head` → `Usage { message: "detached HEAD cannot be pushed by gww" }`.
4. `mapped = store.get_project_by_path(root)?.and_then(|p| p.remote_name)`; `resolve_remote(...)`.
5. `branch_state` using `snapshot.upstream` if present, else `policy.branches.feature.start_from`.
6. `plan_push(&policy, &branch, &remote)`.
7. Return `(plan, snapshot)`. Do not call `upsert_project`.

`execute_push`:

1. `(plan, snapshot) = plan_push_changes(...)`.
2. If `plan.commands.is_empty()` return `ExecuteOutcome { operation_id: String::new(), status: "noop".into(), changed: vec![] }` without writing SQLite.
3. Upsert project, then `execute_plan`.

`list_project_operations`: toplevel, `get_project_by_path`, else `RepositoryNotMapped`, else `list_operations(project_id, limit)` with default limit 20.

`use_cases/mod.rs` add `pub mod push; pub mod ops;`.

- [ ] **Step 4: Run — expect PASS**

Run: `cargo test -p workbench-application --test push --test start_issue`

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/workbench-application
git commit -m "feat(application): plan and execute push; list operations"
```

---

### Task 9: CLI wiring (`gww`)

**Files:**
- Modify: `crates/workbench-cli/Cargo.toml`
- Modify: `crates/workbench-cli/src/main.rs`
- Create: `crates/workbench-cli/src/args.rs`
- Create: `crates/workbench-cli/src/data_dir.rs`
- Create: `crates/workbench-cli/src/render.rs`
- Create: `crates/workbench-cli/src/confirm.rs`

**Interfaces:**
- Consumes: all use cases, `SqliteStore`, `ProcessGitClient<StdProcessRunner>`, `FilePolicySource`, `SystemClock`, `UlidGenerator`, `describe_command`, `AppError::exit_code/user_report`
- Produces: CLI commands in spec §6 plus `--remote` on `open`, `issue start`, and `push`

- [ ] **Step 1: Write failing CLI unit tests for data_dir and clap parse**

`data_dir.rs`:

```rust
use std::path::PathBuf;

pub fn resolve_data_dir(vars: impl Fn(&str) -> Option<String>) -> PathBuf {
    if let Some(dir) = vars("GWW_DATA_DIR") {
        return PathBuf::from(dir);
    }
    if let Some(xdg) = vars("XDG_DATA_HOME") {
        return PathBuf::from(xdg).join("github-workbench");
    }
    if let Some(local) = vars("LOCALAPPDATA") {
        return PathBuf::from(local).join("github-workbench");
    }
    let home = vars("HOME").unwrap_or_else(|| ".".into());
    PathBuf::from(home).join(".local/share/github-workbench")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vars<'a>(pairs: &'a [(&str, &str)]) -> impl Fn(&str) -> Option<String> + 'a {
        |key| {
            pairs
                .iter()
                .find(|(k, _)| *k == key)
                .map(|(_, v)| (*v).to_string())
        }
    }

    #[test]
    fn gww_data_dir_wins() {
        let path = resolve_data_dir(vars(&[
            ("GWW_DATA_DIR", "/tmp/gww"),
            ("XDG_DATA_HOME", "/xdg"),
            ("HOME", "/home/dev"),
        ]));
        assert_eq!(path, PathBuf::from("/tmp/gww"));
    }

    #[test]
    fn xdg_data_home_used() {
        let path = resolve_data_dir(vars(&[("XDG_DATA_HOME", "/xdg"), ("HOME", "/home/dev")]));
        assert_eq!(path, PathBuf::from("/xdg/github-workbench"));
    }

    #[test]
    fn windows_localappdata_used() {
        let path = resolve_data_dir(vars(&[("LOCALAPPDATA", "C:\\Users\\dev\\AppData\\Local")]));
        assert_eq!(
            path,
            PathBuf::from("C:\\Users\\dev\\AppData\\Local").join("github-workbench")
        );
    }

    #[test]
    fn home_fallback() {
        let path = resolve_data_dir(vars(&[("HOME", "/home/dev")]));
        assert_eq!(path, PathBuf::from("/home/dev/.local/share/github-workbench"));
    }
}
```

Production `main` calls `resolve_data_dir(|k| std::env::var(k).ok())`.

Clap tests at the bottom of `args.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn parses_issue_start() {
        let cli = Cli::try_parse_from([
            "gww",
            "issue",
            "start",
            "42",
            "--title",
            "Add resumable uploads",
            "--yes",
        ])
        .unwrap();
        match cli.command {
            Some(Commands::Issue {
                command: IssueCommands::Start { number, title, yes, .. },
            }) => {
                assert_eq!(number, 42);
                assert_eq!(title, "Add resumable uploads");
                assert!(yes);
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn parses_push_plan() {
        let cli = Cli::try_parse_from(["gww", "push", "--plan"]).unwrap();
        match cli.command {
            Some(Commands::Push { plan, yes, .. }) => {
                assert!(plan);
                assert!(!yes);
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn parses_status_json() {
        let cli = Cli::try_parse_from(["gww", "status", "--json"]).unwrap();
        match cli.command {
            Some(Commands::Status { json }) => assert!(json),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn issue_start_requires_title() {
        assert!(Cli::try_parse_from(["gww", "issue", "start", "42"]).is_err());
    }
}
```

- [ ] **Step 2: Run — expect FAIL**

Run: `cargo test -p workbench-cli`

- [ ] **Step 3: Implement CLI**

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
workbench-git = { workspace = true }
workbench-storage = { workspace = true }
clap = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }

[dev-dependencies]
tempfile = { workspace = true }
```

Add to root `Cargo.toml` `[workspace.dependencies]`:

```toml
workbench-git = { path = "crates/workbench-git" }
workbench-storage = { path = "crates/workbench-storage" }
```

`args.rs` (`clap` derive):

```rust
#[derive(Parser)]
#[command(name = "gww", version, about = "GitHub Workflow Workbench")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    Open {
        path: PathBuf,
        #[arg(long)]
        remote: Option<String>,
    },
    Status {
        #[arg(long)]
        json: bool,
    },
    Issue {
        #[command(subcommand)]
        command: IssueCommands,
    },
    Push {
        #[arg(long)]
        plan: bool,
        #[arg(long)]
        yes: bool,
        #[arg(long)]
        remote: Option<String>,
    },
    Ops {
        #[command(subcommand)]
        command: OpsCommands,
    },
}

#[derive(Subcommand)]
pub enum IssueCommands {
    Start {
        number: u64,
        #[arg(long)]
        title: String,
        #[arg(long)]
        yes: bool,
        #[arg(long)]
        remote: Option<String>,
    },
}

#[derive(Subcommand)]
pub enum OpsCommands {
    List,
}
```

`render.rs`: human plan printer (summary, risk lowercase, preconditions, `describe_command` lines, rationale, findings). Status human printer. `--json` uses serde_json on a dedicated `StatusJson` struct (root, branch, detached, head_oid, dirty, dirty_paths, ahead, behind, upstream, remotes, selected_remote, policy_source, findings, recommended_next_action). Ops list printer.

`confirm.rs`: if `--yes`, true. If stdin is not a TTY (`std::io::IsTerminal`), return `AppError::Usage { message: "refusing to execute without --yes because stdin is not a TTY" }`. Else print plan and `Proceed? [y/N] `, accept `y`/`yes` case-insensitive.

`main.rs`: construct adapters; `SqliteStore::open(&data_dir.join("workbench.db"))` creating the directory with `create_dir_all`. Map `AppError` to stderr `user_report()` and `process::exit(code)`. Clap parse errors already exit 2. Declined confirmation: print `Aborted.` and exit 1.

`--plan` on push prints the plan and exits 0 (even for nothing-to-push). `--plan` and `--yes` together: `--plan` wins (no execute).

`open` prints repository root, remotes, selected remote, policy source, project id.

Issue start: plan, confirm, execute, print operation id and created branch name.

Push: plan or confirm+execute; dirty tree prints user_report exit 1.

`ops list`: print operations and steps.

Keep `gww --version` / `-V` via clap.

- [ ] **Step 4: Run — expect PASS**

Run: `cargo test -p workbench-cli`
Run: `cargo run -p workbench-cli -- --version`

Expected: tests PASS; prints `gww 0.1.0`.

- [ ] **Step 5: Commit**

```bash
git add crates/workbench-cli Cargo.lock
git commit -m "feat(cli): wire gww open, status, issue start, push, and ops list"
```

---

### Task 10: End-to-end happy path, docs, and CI gates

**Files:**
- Create: `crates/workbench-cli/tests/cli_happy_path.rs`
- Modify: `README.md`
- Modify: `docs/architecture.md`
- Modify: `crates/workbench-application/README.md`
- Modify: `.github/workflows/ci.yml` (add `git --version` before tests)

**Interfaces:**
- Consumes: `gww` binary via `env!("CARGO_BIN_EXE_gww")`, real Git, temp `GWW_DATA_DIR`
- Produces: exit-criterion coverage; docs that Phase 2 is CLI-complete

- [ ] **Step 1: Write the failing e2e test**

Harness (inline in the test file): isolated Git config as in Task 4; bare remote + clone; `GWW_DATA_DIR` temp; `HOME` / `GIT_CONFIG_*` inherited by the child via `Command::env`.

Flow:

1. `gww open <work>` exit 0; stdout contains toplevel path.
2. Invalid policy: write `.github-workbench.yml` with `typo-field: true`, `gww open` exit 2, then restore/remove file.
3. `gww issue start 42 --title "Add resumable uploads" --yes` exit 0; `git -C work branch --show-current` is `feature/42-add-resumable-uploads`.
4. Commit a file with the isolated git identity.
5. `gww push --plan` exit 0; stdout contains the remote name and branch refspec; does not create the remote branch yet (`git ls-remote` has no feature ref).
6. Dirty the tree, `gww push --yes` exit 1, stdout/stderr mentions dirty; restore.
7. `gww push --yes` exit 0; remote has the feature ref.
8. `gww status --json` parses; `recommended_next_action` is non-empty; `dirty` false.
9. `gww ops list` shows `create-branch-from-issue` and `push` with succeeded steps.
10. `gww push --plan` after a successful push contains `Nothing to push`.

- [ ] **Step 2: Run — expect FAIL**

Run: `cargo test -p workbench-cli --test cli_happy_path -- --nocapture`

Expected: FAIL with missing binary commands or missing `GWW_DATA_DIR` directory until Task 9's `create_dir_all` and the Task 3 `GIT_CONFIG_*` env whitelist are in place. The e2e test must spawn `gww` with `HOME`, `GIT_CONFIG_NOSYSTEM`, `GIT_CONFIG_GLOBAL`, and `GWW_DATA_DIR` set on the child process; `StdProcessRunner` copies those Git config keys from the CLI process environment.

- [ ] **Step 3: Update docs**

`README.md` status line: Phase 2 — local repository CLI vertical slice. Document:

```bash
gww open .
gww status
gww issue start 42 --title "Add resumable uploads"
gww push --plan
gww push --yes
gww ops list
```

`GWW_DATA_DIR`, `GWW_GIT_PROGRAM`. Point at Phase 2 spec + this plan.

`docs/architecture.md`: Phase 2 implements application use cases, Git process adapter, SQLite journal, and `gww`. GitHub adapter remains a stub. Mention argv-only execution and GWW_DATA_DIR.

Application/git/storage READMEs: replace “Phase 2+ stub” with what each crate actually does.

CI: before Test, `git --version`.

- [ ] **Step 4: Run full gates**

```bash
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo run -p workbench-cli -- --version
```

Expected: all succeed; `gww 0.1.0`.

- [ ] **Step 5: Exit-criterion checklist**

- [ ] `gww open` identifies the repository and records the project
- [ ] `gww issue start <n> --title …` creates a GitHub Flow–compliant branch after plan confirmation / `--yes`
- [ ] `gww push --plan` shows the intended remote ref update
- [ ] `gww push --yes` publishes the branch when the tree is clean and commits exist
- [ ] Dirty tree blocks push
- [ ] `gww ops list` shows journaled steps
- [ ] No desktop UI; no `gh`; no `--force`
- [ ] Invalid policy does not mutate Git or SQLite

- [ ] **Step 6: Commit**

```bash
git add crates/workbench-cli README.md docs crates/workbench-application/README.md crates/workbench-git/README.md crates/workbench-storage/README.md .github/workflows/ci.yml
git commit -m "test(cli): add real-git happy path and update Phase 2 docs"
```

---

## Self-review (plan vs spec)

| Spec requirement | Task |
|---|---|
| CLI only; no Tauri/React | Global + Task 9 |
| Process Git client, argv only, no libgit2 | Tasks 3–4 |
| resolve toplevel, status, fetch, create branch, checkout, push, ahead/behind | Task 4 (`checkout -b` / existing checkout) |
| SQLite `projects`, `operations`, `operation_steps` + migrations | Task 5 |
| Ports: GitClient, OperationStore, ProcessRunner, Clock, IdGenerator, PolicySource | Task 2 |
| Use cases: open, status, start issue, plan/execute push, list ops | Tasks 6–8 |
| `gww` commands in §6 | Task 9 |
| Snapshot before mutation; journal pending→running→succeeded/failed | Task 7 |
| No `--force` / `--force-with-lease` | Tasks 3–4, 8 |
| `gww push --plan` dry-run | Task 9 |
| Confirmation or `--yes` | Task 9 |
| Dirty-tree push block | Task 8, 10 |
| Remote via mapping / sole remote / `--remote`, not assumed `origin` | Tasks 1–2, 6–9 |
| Manual issue number + title | Task 9 |
| `GWW_DATA_DIR` / platform data dir | Task 9 |
| Invalid policy → error, no mutation | Tasks 6, 10 |
| User-facing errors: failed / changed / unchanged / retry / remediation | Task 2 |
| Exit codes 0/1/2/3 | Tasks 2, 9 |
| Git integration tests with isolated config + temp bare remote | Tasks 4, 10 |
| Storage CRUD tests | Task 5 |
| Application tests with fakes + one real-Git happy path | Tasks 6–8, 10 |
| `workbench-github` stub | Global (untouched) |
| High-risk ops not in allowlist | Task 7 exhaustive `GitCommand` match |
| Nothing to push explained | Tasks 1, 8, 10 |
| Redacted bounded step output | Tasks 2, 5, 7 |
| CI fmt/clippy/test with Git available | Task 10 |

**Placeholder scan:** No TBD/TODO/“implement later” left in task steps. Locked persistence rule: `plan_*` functions never write SQLite; only `open_repository` and `execute_*` upsert. Invalid policy writes nothing. `status` never writes SQLite. Empty push plans are noops with no journal row.

**Type consistency:** `plan_create_branch_from_issue(..., remote: &str)`, `plan_push(policy, current, remote)`, `AppError`, `GitClient` method names, `SqliteStore::open`, `resolve_remote`, `ExecuteOutcome`, and CLI flags are named the same in later tasks as in earlier ones.

**YAGNI held:** no commit UX, no worktrees, no rebase, no PR, no `gh`, no Tokio, no force-with-lease, no output spill files (bounded TEXT only).
