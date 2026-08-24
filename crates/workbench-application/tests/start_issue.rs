use std::cell::RefCell;
use std::path::{Path, PathBuf};

use workbench_application::executor::execute_plan;
use workbench_application::fakes::{FakeClock, FakeGit, FakeIds, FakePolicy, FakeStore};
use workbench_application::ports::{OperationStore, ProjectRecord};
use workbench_application::use_cases::start_issue::{execute_start_issue, plan_start_issue};
use workbench_application::AppError;
use workbench_domain::operations::plan::{GitCommand, OperationPlan, RiskClass, StepStatus};
use workbench_domain::policy::PolicyFinding;
use workbench_domain::repository::{BranchState, Remote, RepositorySnapshot};

const NOW: &str = "2026-08-24T00:00:00Z";
const ROOT: &str = "/tmp/repo";

fn remote(name: &str, repo: &str) -> Remote {
    Remote {
        name: name.into(),
        url: format!("git@github.com:acme/{repo}.git"),
    }
}

fn snapshot(branch: &str, remotes: Vec<Remote>) -> RepositorySnapshot {
    RepositorySnapshot {
        root: ROOT.into(),
        branch: Some(branch.into()),
        detached_head: false,
        head_oid: Some("abc".into()),
        dirty_paths: vec![],
        remotes,
        selected_remote: None,
        upstream: None,
    }
}

fn branch(name: &str) -> BranchState {
    BranchState {
        name: name.into(),
        head_oid: Some("abc".into()),
        upstream: None,
        base_branch: Some("main".into()),
        ahead: 0,
        behind: 0,
        dirty_paths: vec![],
        is_protected: name == "main",
    }
}

fn fake_git(snapshot: RepositorySnapshot, branch: BranchState) -> FakeGit {
    FakeGit {
        toplevel: PathBuf::from(ROOT),
        snapshot: RefCell::new(snapshot),
        branch: RefCell::new(branch),
        executed: RefCell::new(vec![]),
        fail_kind: RefCell::new(None),
    }
}

fn plan(risk: RiskClass, commands: Vec<GitCommand>) -> OperationPlan {
    OperationPlan {
        id: ulid::Ulid::nil(),
        kind: "test-plan".into(),
        risk,
        summary: "Test plan".into(),
        rationale: vec![],
        commands,
        preconditions: vec![],
        findings: Vec::<PolicyFinding>::new(),
    }
}

fn all_commands() -> Vec<GitCommand> {
    vec![
        GitCommand::Fetch {
            remote: "github".into(),
        },
        GitCommand::CreateBranch {
            name: "feature/42-add-resumable-uploads".into(),
            start_point: "main".into(),
        },
        GitCommand::PushRef {
            remote: "github".into(),
            local_ref: "feature/42-add-resumable-uploads".into(),
            remote_ref: "feature/42-add-resumable-uploads".into(),
            set_upstream: true,
        },
    ]
}

fn mapped_project(remote_name: &str) -> ProjectRecord {
    ProjectRecord {
        id: "project-1".into(),
        local_path: ROOT.into(),
        github_host: Some("github.com".into()),
        owner: Some("acme".into()),
        repo: Some("widgets".into()),
        remote_name: Some(remote_name.into()),
        created_at: NOW.into(),
        updated_at: NOW.into(),
    }
}

#[test]
fn plan_start_issue_on_main_only_plans_branch_creation_without_store_mutation() {
    let git = fake_git(
        snapshot("main", vec![remote("github", "widgets")]),
        branch("main"),
    );
    let store = FakeStore::new();

    let (plan, snapshot, _) = plan_start_issue(
        &git,
        &store,
        &FakePolicy { yaml: None },
        Path::new(ROOT),
        42,
        "Add resumable uploads",
        None,
    )
    .unwrap();

    assert!(plan.summary.contains("feature/42-add-resumable-uploads"));
    assert!(matches!(
        plan.commands.as_slice(),
        [GitCommand::CreateBranch { name, start_point }]
            if name == "feature/42-add-resumable-uploads" && start_point == "main"
    ));
    assert_eq!(snapshot.selected_remote.as_deref(), Some("github"));
    assert!(store.projects.lock().unwrap().is_empty());
}

#[test]
fn plan_start_issue_uses_mapped_remote_for_fetch() {
    let remotes = vec![
        remote("origin", "widgets-mirror"),
        remote("github", "widgets"),
    ];
    let git = fake_git(snapshot("topic", remotes), branch("topic"));
    let store = FakeStore::new();
    store
        .projects
        .lock()
        .unwrap()
        .push(mapped_project("github"));

    let (plan, _, _) = plan_start_issue(
        &git,
        &store,
        &FakePolicy { yaml: None },
        Path::new(ROOT),
        42,
        "Add resumable uploads",
        None,
    )
    .unwrap();

    assert!(matches!(
        plan.commands.first(),
        Some(GitCommand::Fetch { remote }) if remote == "github"
    ));
    assert_eq!(store.projects.lock().unwrap().len(), 1);
}

#[test]
fn plan_start_issue_remote_flag_selects_fetch_remote() {
    let remotes = vec![
        remote("origin", "widgets-mirror"),
        remote("github", "widgets"),
    ];
    let git = fake_git(snapshot("topic", remotes), branch("topic"));
    let store = FakeStore::new();

    let (plan, _, _) = plan_start_issue(
        &git,
        &store,
        &FakePolicy { yaml: None },
        Path::new(ROOT),
        42,
        "Add resumable uploads",
        Some("github"),
    )
    .unwrap();

    assert!(matches!(
        plan.commands.first(),
        Some(GitCommand::Fetch { remote }) if remote == "github"
    ));
    assert!(store.projects.lock().unwrap().is_empty());
}

#[test]
fn execute_plan_runs_allowlisted_commands_and_journals_success() {
    let snapshot = snapshot("topic", vec![remote("github", "widgets")]);
    let git = fake_git(snapshot.clone(), branch("topic"));
    let store = FakeStore::new();
    let commands = all_commands();
    let plan = plan(RiskClass::Low, commands.clone());

    let outcome = execute_plan(
        &git,
        &store,
        &FakeClock(NOW.into()),
        &FakeIds::new(),
        "project-1",
        &snapshot,
        &plan,
    )
    .unwrap();

    assert_eq!(outcome.status, "succeeded");
    assert_eq!(outcome.changed.len(), commands.len());
    assert_eq!(*git.executed.borrow(), commands);
    let operations = store.operations.lock().unwrap();
    assert_eq!(operations.len(), 1);
    assert_eq!(operations[0].status, "succeeded");
    assert_eq!(
        operations[0]
            .steps
            .iter()
            .map(|step| step.status)
            .collect::<Vec<_>>(),
        vec![
            StepStatus::Succeeded,
            StepStatus::Succeeded,
            StepStatus::Succeeded
        ]
    );
    assert_eq!(
        operations[0]
            .steps
            .iter()
            .map(|step| step.sequence)
            .collect::<Vec<_>>(),
        vec![1, 2, 3]
    );
}

#[test]
fn create_branch_failure_marks_later_steps_skipped_and_is_not_retry_safe() {
    let snapshot = snapshot("topic", vec![remote("github", "widgets")]);
    let git = fake_git(snapshot.clone(), branch("topic"));
    *git.fail_kind.borrow_mut() = Some("create-branch".into());
    let store = FakeStore::new();
    let plan = plan(RiskClass::Low, all_commands());

    let error = execute_plan(
        &git,
        &store,
        &FakeClock(NOW.into()),
        &FakeIds::new(),
        "project-1",
        &snapshot,
        &plan,
    )
    .unwrap_err();

    let (changed, unchanged, retry_safe) = match error {
        AppError::OperationFailed {
            changed,
            unchanged,
            retry_safe,
            ..
        } => (changed, unchanged, retry_safe),
        other => panic!("expected OperationFailed, got {other:?}"),
    };
    assert_eq!(changed.len(), 1);
    assert_eq!(unchanged.len(), 1);
    assert!(!retry_safe);
    assert_eq!(
        *git.executed.borrow(),
        all_commands().into_iter().take(2).collect::<Vec<_>>()
    );
    let operations = store.operations.lock().unwrap();
    assert_eq!(operations[0].status, "failed");
    assert_eq!(
        operations[0]
            .steps
            .iter()
            .map(|step| step.status)
            .collect::<Vec<_>>(),
        vec![
            StepStatus::Succeeded,
            StepStatus::Failed,
            StepStatus::Skipped
        ]
    );
}

#[test]
fn fetch_failure_is_retry_safe_and_changes_nothing() {
    let snapshot = snapshot("topic", vec![remote("github", "widgets")]);
    let git = fake_git(snapshot.clone(), branch("topic"));
    *git.fail_kind.borrow_mut() = Some("fetch".into());
    let store = FakeStore::new();
    let plan = plan(RiskClass::Low, all_commands());

    let error = execute_plan(
        &git,
        &store,
        &FakeClock(NOW.into()),
        &FakeIds::new(),
        "project-1",
        &snapshot,
        &plan,
    )
    .unwrap_err();

    assert!(matches!(
        error,
        AppError::OperationFailed {
            ref changed,
            retry_safe: true,
            ..
        } if changed.is_empty()
    ));
    assert_eq!(git.executed.borrow().len(), 1);
}

#[test]
fn push_failure_after_started_is_not_retry_safe() {
    let snapshot = snapshot("topic", vec![remote("github", "widgets")]);
    let git = fake_git(snapshot.clone(), branch("topic"));
    *git.fail_kind.borrow_mut() = Some("push-ref".into());
    let store = FakeStore::new();
    let plan = plan(RiskClass::Low, all_commands());

    let error = execute_plan(
        &git,
        &store,
        &FakeClock(NOW.into()),
        &FakeIds::new(),
        "project-1",
        &snapshot,
        &plan,
    )
    .unwrap_err();

    assert!(matches!(
        error,
        AppError::OperationFailed {
            ref changed,
            retry_safe: false,
            ..
        } if changed.len() == 2
    ));
}

#[test]
fn high_risk_plan_is_rejected_before_git_or_store_calls() {
    let snapshot = snapshot("topic", vec![remote("github", "widgets")]);
    let git = fake_git(snapshot.clone(), branch("topic"));
    let store = FakeStore::new();
    let plan = plan(RiskClass::High, all_commands());

    let error = execute_plan(
        &git,
        &store,
        &FakeClock(NOW.into()),
        &FakeIds::new(),
        "project-1",
        &snapshot,
        &plan,
    )
    .unwrap_err();

    assert_eq!(
        error,
        AppError::Usage {
            message: "high-risk operations are not allowed in Phase 2".into()
        }
    );
    assert!(git.executed.borrow().is_empty());
    assert!(store.operations.lock().unwrap().is_empty());
}

#[test]
fn empty_plan_is_noop_without_journal_row() {
    let snapshot = snapshot("main", vec![remote("github", "widgets")]);
    let git = fake_git(snapshot.clone(), branch("main"));
    let store = FakeStore::new();
    let plan = plan(RiskClass::Low, vec![]);

    let outcome = execute_plan(
        &git,
        &store,
        &FakeClock(NOW.into()),
        &FakeIds::new(),
        "project-1",
        &snapshot,
        &plan,
    )
    .unwrap();

    assert_eq!(outcome.operation_id, "");
    assert_eq!(outcome.status, "noop");
    assert!(outcome.changed.is_empty());
    assert!(git.executed.borrow().is_empty());
    assert!(store.operations.lock().unwrap().is_empty());
}

#[test]
fn invalid_policy_does_not_write_store() {
    let git = fake_git(
        snapshot("main", vec![remote("github", "widgets")]),
        branch("main"),
    );
    let store = FakeStore::new();

    let error = execute_start_issue(
        &git,
        &store,
        &FakePolicy {
            yaml: Some(
                "schema-version: 1\nstrategy:\n  preset: github-flow\n  default-branch: main\ntypo-field: true\n"
                    .into(),
            ),
        },
        &FakeClock(NOW.into()),
        &FakeIds::new(),
        Path::new(ROOT),
        42,
        "Add resumable uploads",
        None,
    )
    .unwrap_err();

    assert!(matches!(error, AppError::Domain(_)));
    assert!(store.projects.lock().unwrap().is_empty());
    assert!(store.operations.lock().unwrap().is_empty());
}

#[test]
fn execute_start_issue_upserts_project_then_executes_plan() {
    let git = fake_git(
        snapshot("main", vec![remote("github", "widgets")]),
        branch("main"),
    );
    let store = FakeStore::new();

    let outcome = execute_start_issue(
        &git,
        &store,
        &FakePolicy { yaml: None },
        &FakeClock(NOW.into()),
        &FakeIds::new(),
        Path::new(ROOT),
        42,
        "Add resumable uploads",
        None,
    )
    .unwrap();

    assert_eq!(outcome.status, "succeeded");
    let project = store.get_project_by_path(Path::new(ROOT)).unwrap().unwrap();
    assert_eq!(project.remote_name.as_deref(), Some("github"));
    assert_eq!(project.owner.as_deref(), Some("acme"));
    assert_eq!(project.repo.as_deref(), Some("widgets"));
    assert_eq!(store.operations.lock().unwrap()[0].project_id, project.id);
}

fn command_is_compile_time_allowlisted(command: &GitCommand) -> &'static str {
    match command {
        GitCommand::Fetch { .. } => "fetch",
        GitCommand::CreateBranch { .. } => "create-branch",
        GitCommand::PushRef { .. } => "push-ref",
    }
}

#[test]
fn git_command_enum_is_the_exhaustive_executor_allowlist() {
    assert_eq!(
        all_commands()
            .iter()
            .map(command_is_compile_time_allowlisted)
            .collect::<Vec<_>>(),
        vec!["fetch", "create-branch", "push-ref"]
    );
}
