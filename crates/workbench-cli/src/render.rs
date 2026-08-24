use std::fmt::Write as _;

use serde::Serialize;
use workbench_application::action_tests::{
    RemoteTestResult, RemoteTestSessionPlan, TestSessionStatus,
};
use workbench_application::ports::{CleanupItemRecord, OperationRecord, TestSessionRecord};
use workbench_application::use_cases::action_discovery::ActionTestCatalog;
use workbench_application::use_cases::status::StatusOutcome;
use workbench_application::AppError;
use workbench_domain::operations::plan::{OperationPlan, RiskClass, StepStatus};
use workbench_domain::policy::{PolicyFinding, Severity};
use workbench_domain::repository::Remote;
use workbench_domain::testing::{ActionRuntime, RESULT_ARTIFACT_NAME};
use workbench_git::describe_command;

pub fn render_plan(plan: &OperationPlan) -> String {
    let mut output = String::new();
    let _ = writeln!(output, "{}", plan.summary);
    let _ = writeln!(output, "Risk: {}", risk_label(plan.risk));
    write_list(&mut output, "Preconditions", &plan.preconditions);

    let _ = writeln!(output, "Commands:");
    if plan.commands.is_empty() {
        let _ = writeln!(output, "  (none)");
    } else {
        for command in &plan.commands {
            let _ = writeln!(output, "  {}", describe_command(command));
        }
    }

    write_list(&mut output, "Rationale", &plan.rationale);
    write_findings(&mut output, &plan.findings);
    output.trim_end().to_string()
}

pub fn render_remote_test_plan(plan: &RemoteTestSessionPlan) -> String {
    let mut output = render_plan(&plan.git_plan);
    let _ = writeln!(output);
    let _ = writeln!(output, "Generated workflow: {}", plan.workflow_path);
    let _ = writeln!(
        output,
        "Generated branch: {}",
        plan.cleanup_identity.ref_name
    );
    let _ = writeln!(output, "Repository: {}/{}", plan.owner, plan.repo);
    let _ = writeln!(output, "Expected runner: {}", plan.test_plan.runner);
    let _ = writeln!(output, "Artifact: {RESULT_ARTIFACT_NAME}");
    let _ = writeln!(output, "Estimated jobs: 1");
    let _ = writeln!(
        output,
        "Cleanup policy: Success: {}h; failure: {}h",
        plan.successful_ref_retention.0, plan.failed_ref_retention.0
    );
    output.trim_end().to_string()
}

pub fn render_action_catalog(catalog: &ActionTestCatalog) -> String {
    let mut output = String::new();
    let _ = writeln!(output, "Actions:");
    if catalog.actions.is_empty() {
        let _ = writeln!(output, "  (none)");
    } else {
        for action in &catalog.actions {
            let runtime = match &action.definition.runtime {
                ActionRuntime::Composite => "composite",
                ActionRuntime::Unsupported { using } => using,
            };
            let support = if action.supported {
                "supported"
            } else {
                "unsupported"
            };
            let _ = writeln!(
                output,
                "  {}  {}  {}  {}",
                action.definition.name, runtime, support, action.definition.manifest_path
            );
            if let Some(warning) = &action.warning {
                let _ = writeln!(output, "    Warning: {warning}");
            }
        }
    }
    let _ = writeln!(output, "Tests:");
    if catalog.tests.is_empty() {
        let _ = writeln!(output, "  (none)");
    } else {
        for test in &catalog.tests {
            let _ = writeln!(output, "  {}  {}", test.name, test.path.display());
        }
    }
    output.trim_end().to_string()
}

pub fn render_remote_test_result(result: &RemoteTestResult) -> String {
    let mut output = String::new();
    let _ = writeln!(output, "Session id: {}", result.session_id);
    let _ = writeln!(output, "Run URL: {}", result.run_url);
    let _ = writeln!(output, "Conclusion: {}", result.conclusion);
    let _ = writeln!(
        output,
        "Assertions: {}",
        if result.passed { "passed" } else { "failed" }
    );
    let _ = writeln!(
        output,
        "Manifest: {}",
        result
            .manifest_path
            .as_ref()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| "(not downloaded)".into())
    );
    let _ = writeln!(output, "Logs: {}", result.logs_path.display());
    let _ = writeln!(
        output,
        "Cleanup: run `gww cleanup list` to inspect the temporary ref."
    );
    output.trim_end().to_string()
}

pub fn render_status(outcome: &StatusOutcome) -> String {
    let snapshot = &outcome.snapshot;
    let mut output = String::new();
    let _ = writeln!(output, "Root: {}", snapshot.root);
    let _ = writeln!(output, "Branch: {}", outcome.branch.name);
    let _ = writeln!(output, "Detached: {}", snapshot.detached_head);
    let _ = writeln!(
        output,
        "HEAD: {}",
        snapshot.head_oid.as_deref().unwrap_or("(unborn)")
    );
    let _ = writeln!(
        output,
        "Working tree: {}",
        if snapshot.dirty_paths.is_empty() {
            "clean"
        } else {
            "dirty"
        }
    );
    if !snapshot.dirty_paths.is_empty() {
        for path in &snapshot.dirty_paths {
            let _ = writeln!(output, "  {path}");
        }
    }
    let _ = writeln!(
        output,
        "Ahead/behind: {}/{}",
        outcome.branch.ahead, outcome.branch.behind
    );
    let _ = writeln!(
        output,
        "Upstream: {}",
        outcome.branch.upstream.as_deref().unwrap_or("(none)")
    );
    let _ = writeln!(output, "Remotes:");
    if snapshot.remotes.is_empty() {
        let _ = writeln!(output, "  (none)");
    } else {
        for remote in &snapshot.remotes {
            let selected = if snapshot.selected_remote.as_deref() == Some(&remote.name) {
                " (selected)"
            } else {
                ""
            };
            let _ = writeln!(output, "  {}: {}{selected}", remote.name, remote.url);
        }
    }
    let _ = writeln!(output, "Policy source: {}", outcome.policy_source);
    write_findings(&mut output, &outcome.findings);
    let _ = writeln!(
        output,
        "Recommended next action: {}",
        outcome.recommended_next_action
    );
    output.trim_end().to_string()
}

#[derive(Serialize)]
struct StatusJson<'a> {
    root: &'a str,
    branch: &'a str,
    detached: bool,
    head_oid: &'a Option<String>,
    dirty: bool,
    dirty_paths: &'a [String],
    ahead: u64,
    behind: u64,
    upstream: &'a Option<String>,
    remotes: &'a [Remote],
    selected_remote: &'a Option<String>,
    policy_source: &'a str,
    findings: &'a [PolicyFinding],
    recommended_next_action: &'a str,
}

pub fn render_status_json(outcome: &StatusOutcome) -> Result<String, AppError> {
    let status = StatusJson {
        root: &outcome.snapshot.root,
        branch: &outcome.branch.name,
        detached: outcome.snapshot.detached_head,
        head_oid: &outcome.snapshot.head_oid,
        dirty: !outcome.snapshot.dirty_paths.is_empty(),
        dirty_paths: &outcome.snapshot.dirty_paths,
        ahead: outcome.branch.ahead,
        behind: outcome.branch.behind,
        upstream: &outcome.branch.upstream,
        remotes: &outcome.snapshot.remotes,
        selected_remote: &outcome.snapshot.selected_remote,
        policy_source: outcome.policy_source,
        findings: &outcome.findings,
        recommended_next_action: &outcome.recommended_next_action,
    };
    serde_json::to_string_pretty(&status).map_err(|error| AppError::Storage {
        detail: format!("could not serialize repository status: {error}"),
    })
}

pub fn render_operations(operations: &[OperationRecord]) -> String {
    if operations.is_empty() {
        return "No operations recorded.".into();
    }

    let mut output = String::new();
    for operation in operations {
        let _ = writeln!(
            output,
            "{}  {}  {}",
            operation.id, operation.kind, operation.status
        );
        for step in &operation.steps {
            let _ = writeln!(
                output,
                "  {}. {}  {}",
                step.sequence,
                step.kind,
                step_status_label(step.status)
            );
        }
    }
    output.trim_end().to_string()
}

pub fn render_sessions(sessions: &[TestSessionRecord]) -> String {
    if sessions.is_empty() {
        return "No remote test sessions recorded.".into();
    }

    let mut output = String::new();
    let _ = writeln!(
        output,
        "{:<11} {:<12} {:<9} REMOTE REF",
        "SESSION", "STATUS", "RUN"
    );
    for session in sessions {
        let run_id = session
            .run_id
            .map(|value| value.to_string())
            .unwrap_or_else(|| "-".into());
        let _ = writeln!(
            output,
            "{:<11} {:<12} {:<9} {}",
            session.session_id,
            session_status_label(&session.status),
            run_id,
            session.remote_ref
        );
    }
    output.trim_end().to_string()
}

pub fn render_cleanup(items: &[CleanupItemRecord]) -> String {
    if items.is_empty() {
        return "No cleanup items recorded.".into();
    }

    let mut output = String::new();
    let _ = writeln!(
        output,
        "{:<12} {:<12} {:<22} RESOURCE",
        "ITEM", "STATUS", "DUE"
    );
    for item in items {
        let _ = writeln!(
            output,
            "{:<12} {:<12} {:<22} {}",
            item.id, item.status, item.due_at, item.resource_id
        );
    }
    output.trim_end().to_string()
}

fn write_list(output: &mut String, heading: &str, items: &[String]) {
    let _ = writeln!(output, "{heading}:");
    if items.is_empty() {
        let _ = writeln!(output, "  (none)");
    } else {
        for item in items {
            let _ = writeln!(output, "  - {item}");
        }
    }
}

fn write_findings(output: &mut String, findings: &[PolicyFinding]) {
    let _ = writeln!(output, "Findings:");
    if findings.is_empty() {
        let _ = writeln!(output, "  (none)");
    } else {
        for finding in findings {
            let _ = writeln!(
                output,
                "  [{}] {}",
                severity_label(finding.severity),
                finding.message
            );
            let _ = writeln!(output, "    Remediation: {}", finding.remediation);
        }
    }
}

fn risk_label(risk: RiskClass) -> &'static str {
    match risk {
        RiskClass::Low => "low",
        RiskClass::Medium => "medium",
        RiskClass::High => "high",
    }
}

fn severity_label(severity: Severity) -> &'static str {
    match severity {
        Severity::Warning => "warning",
        Severity::Blocker => "blocker",
    }
}

fn step_status_label(status: StepStatus) -> &'static str {
    match status {
        StepStatus::Pending => "pending",
        StepStatus::Running => "running",
        StepStatus::Succeeded => "succeeded",
        StepStatus::Failed => "failed",
        StepStatus::Skipped => "skipped",
        StepStatus::CompensationNeeded => "compensation-needed",
    }
}

fn session_status_label(status: &TestSessionStatus) -> &'static str {
    match status {
        TestSessionStatus::Planned => "planned",
        TestSessionStatus::Pushed => "pushed",
        TestSessionStatus::Queued => "queued",
        TestSessionStatus::InProgress => "in-progress",
        TestSessionStatus::Passed => "passed",
        TestSessionStatus::Failed => "failed",
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::path::PathBuf;

    use super::*;
    use workbench_application::action_tests::{
        CleanupIdentity, RemoteTestSessionPlan, TestSessionStatus,
    };
    use workbench_application::ports::{
        CleanupItemRecord, OperationRecord, StepRecord, TestSessionRecord,
    };
    use workbench_application::use_cases::status::StatusOutcome;
    use workbench_domain::operations::plan::{GitCommand, OperationPlan, RiskClass, StepStatus};
    use workbench_domain::policy::{github_flow_defaults, PolicyFinding, RetentionHours, Severity};
    use workbench_domain::repository::{BranchState, Remote, RepositorySnapshot};
    use workbench_domain::testing::{TestAssertions, TestPermissions, TestPlan};

    fn finding() -> PolicyFinding {
        PolicyFinding {
            rule_id: "branch.protected".into(),
            severity: Severity::Blocker,
            expected: "feature branch".into(),
            actual: "main".into(),
            message: "Main is protected.".into(),
            remediation: "Create a feature branch.".into(),
        }
    }

    #[test]
    fn plan_includes_all_explanatory_sections_and_git_argv() {
        let plan = OperationPlan {
            id: "01ARZ3NDEKTSV4RRFFQ69G5FAV".parse().unwrap(),
            kind: "create-branch-from-issue".into(),
            risk: RiskClass::Medium,
            summary: "Create an issue branch".into(),
            rationale: vec!["Branches isolate work.".into()],
            commands: vec![GitCommand::CreateBranch {
                name: "feature/42-uploads".into(),
                start_point: "main".into(),
            }],
            preconditions: vec!["Working tree is clean.".into()],
            findings: vec![finding()],
        };

        let output = render_plan(&plan);

        assert!(output.contains("Create an issue branch"));
        assert!(output.contains("Risk: medium"));
        assert!(output.contains("Working tree is clean."));
        assert!(output.contains("git checkout -b feature/42-uploads main --"));
        assert!(output.contains("Branches isolate work."));
        assert!(output.contains("[blocker] Main is protected."));
        assert!(output.contains("Create a feature branch."));
    }

    #[test]
    fn status_json_has_the_authoritative_shape() {
        let outcome = StatusOutcome {
            snapshot: RepositorySnapshot {
                root: "/repo".into(),
                branch: Some("feature/x".into()),
                detached_head: false,
                head_oid: Some("abc123".into()),
                dirty_paths: vec![],
                remotes: vec![Remote {
                    name: "origin".into(),
                    url: "git@github.com:acme/repo.git".into(),
                }],
                selected_remote: Some("origin".into()),
                upstream: Some("origin/feature/x".into()),
            },
            branch: BranchState {
                name: "feature/x".into(),
                head_oid: Some("abc123".into()),
                upstream: Some("origin/feature/x".into()),
                base_branch: Some("main".into()),
                ahead: 2,
                behind: 1,
                dirty_paths: vec![],
                is_protected: false,
            },
            policy: github_flow_defaults(),
            policy_source: "defaults",
            findings: vec![],
            recommended_next_action: "Run gww push --plan".into(),
        };

        let value: serde_json::Value =
            serde_json::from_str(&render_status_json(&outcome).unwrap()).unwrap();

        let object = value.as_object().unwrap();
        let expected = [
            "root",
            "branch",
            "detached",
            "head_oid",
            "dirty",
            "dirty_paths",
            "ahead",
            "behind",
            "upstream",
            "remotes",
            "selected_remote",
            "policy_source",
            "findings",
            "recommended_next_action",
        ];
        assert_eq!(object.len(), expected.len());
        assert!(expected.iter().all(|key| object.contains_key(*key)));
        assert_eq!(value["dirty"], false);
        assert_eq!(value["ahead"], 2);
    }

    #[test]
    fn operations_include_steps() {
        let operations = vec![OperationRecord {
            id: "op-1".into(),
            project_id: "project-1".into(),
            kind: "push".into(),
            status: "succeeded".into(),
            plan_json: "{}".into(),
            started_at: Some("2026-08-24T00:00:00Z".into()),
            completed_at: Some("2026-08-24T00:00:01Z".into()),
            snapshot_json: None,
            steps: vec![StepRecord {
                id: "step-1".into(),
                operation_id: "op-1".into(),
                sequence: 1,
                kind: "push-ref".into(),
                status: StepStatus::Succeeded,
                detail_json: None,
                output_text: None,
            }],
        }];

        let output = render_operations(&operations);

        assert!(output.contains("push"));
        assert!(output.contains("succeeded"));
        assert!(output.contains("1. push-ref"));
    }

    #[test]
    fn remote_test_plan_includes_remote_execution_details() {
        let assertions = TestAssertions {
            conclusion: "success".into(),
            log_contains: vec![],
            log_not_contains: vec![],
        };
        let plan = RemoteTestSessionPlan {
            project_id: "project-1".into(),
            repo_root: PathBuf::from("/repo"),
            owner: "acme".into(),
            repo: "widgets".into(),
            remote: "origin".into(),
            base_sha: "abc123".into(),
            session_id: "01JABC".into(),
            workflow_file_name: "github-workbench-test-01JABC.yml".into(),
            workflow_path: ".github/workflows/github-workbench-test-01JABC.yml".into(),
            workflow_yaml: "name: test".into(),
            test_plan: TestPlan {
                name: "smoke-composite".into(),
                description: None,
                action_path: ".github/actions/smoke".into(),
                runner: "ubuntu-latest".into(),
                timeout_minutes: 15,
                permissions: TestPermissions::default(),
                inputs: BTreeMap::new(),
                environment: BTreeMap::new(),
                assertions: assertions.clone(),
            },
            assertions,
            successful_ref_retention: RetentionHours(0),
            failed_ref_retention: RetentionHours(72),
            cleanup_identity: CleanupIdentity {
                remote: "origin".into(),
                ref_name: "github-workbench/test/01JABC".into(),
                session_id: "01JABC".into(),
            },
            git_plan: OperationPlan {
                id: "01ARZ3NDEKTSV4RRFFQ69G5FAV".parse().unwrap(),
                kind: "remote-action-test".into(),
                risk: RiskClass::Medium,
                summary: "Run remote action test".into(),
                rationale: vec![],
                commands: vec![GitCommand::PushRef {
                    remote: "origin".into(),
                    local_ref: "github-workbench/test/01JABC".into(),
                    remote_ref: "github-workbench/test/01JABC".into(),
                    set_upstream: false,
                }],
                preconditions: vec!["The working tree remains clean.".into()],
                findings: vec![],
            },
        };

        let output = render_remote_test_plan(&plan);

        for expected in [
            "Risk: medium",
            "The working tree remains clean.",
            ".github/workflows/github-workbench-test-01JABC.yml",
            "github-workbench/test/01JABC",
            "git push -- origin github-workbench/test/01JABC:github-workbench/test/01JABC",
            "acme/widgets",
            "ubuntu-latest",
            "github-workbench-result",
            "Estimated jobs: 1",
            "Success: 0h; failure: 72h",
        ] {
            assert!(
                output.contains(expected),
                "remote plan did not contain {expected:?}:\n{output}"
            );
        }
    }

    #[test]
    fn session_and_cleanup_lists_have_stable_tables() {
        let sessions = vec![TestSessionRecord {
            id: "row-1".into(),
            project_id: "project-1".into(),
            session_id: "01JABC".into(),
            commit_sha: "abc123".into(),
            remote_ref: "github-workbench/test/01JABC".into(),
            workflow_name: "workflow.yml".into(),
            run_id: Some(42),
            status: TestSessionStatus::Passed,
            result_json: "{}".into(),
            evidence_dir: None,
            created_at: "2026-08-24T00:00:00Z".into(),
            updated_at: "2026-08-24T00:00:00Z".into(),
        }];
        let cleanup = vec![CleanupItemRecord {
            id: "cleanup-1".into(),
            project_id: "project-1".into(),
            resource_kind: "remote-git-ref".into(),
            resource_id: "origin/github-workbench/test/01JABC".into(),
            expected_identity: "{}".into(),
            due_at: "2026-08-24T00:00:00Z".into(),
            status: "pending".into(),
            created_at: "2026-08-24T00:00:00Z".into(),
            updated_at: "2026-08-24T00:00:00Z".into(),
        }];

        assert_eq!(
            render_sessions(&sessions),
            "SESSION     STATUS       RUN       REMOTE REF\n\
             01JABC      passed       42        github-workbench/test/01JABC"
        );
        assert_eq!(
            render_cleanup(&cleanup),
            "ITEM         STATUS       DUE                    RESOURCE\n\
             cleanup-1    pending      2026-08-24T00:00:00Z   origin/github-workbench/test/01JABC"
        );
    }
}
