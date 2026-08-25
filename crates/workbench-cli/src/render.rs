use std::fmt::Write as _;

use serde::Serialize;
use workbench_application::ports::OperationRecord;
use workbench_application::use_cases::status::StatusOutcome;
use workbench_application::AppError;
use workbench_domain::operations::plan::{OperationPlan, RiskClass, StepStatus};
use workbench_domain::policy::{PolicyFinding, Severity};
use workbench_domain::repository::Remote;
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

#[cfg(test)]
mod tests {
    use super::*;
    use workbench_application::ports::{OperationRecord, StepRecord};
    use workbench_application::use_cases::status::StatusOutcome;
    use workbench_domain::operations::plan::{GitCommand, OperationPlan, RiskClass, StepStatus};
    use workbench_domain::policy::{github_flow_defaults, PolicyFinding, Severity};
    use workbench_domain::repository::{BranchState, Remote, RepositorySnapshot};

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
}
