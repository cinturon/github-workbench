use std::cell::RefCell;
use std::path::{Path, PathBuf};

use workbench_application::fakes::{FakeClock, FakeGit, FakeIds, FakePolicy, FakeStore};
use workbench_application::use_cases::ops::list_project_operations;
use workbench_application::use_cases::push::{execute_push, plan_push_changes};
use workbench_application::AppError;
use workbench_domain::operations::plan::{GitCommand, StepStatus};
use workbench_domain::repository::{BranchState, Remote, RepositorySnapshot};
use workbench_domain::WorkbenchError;

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

fn branch(name: &str, ahead: u64) -> BranchState {
    BranchState {
        name: name.into(),
        head_oid: Some("abc".into()),
        upstream: None,
        base_branch: Some("main".into()),
        ahead,
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
        refs: RefCell::new(Default::default()),
    }
}

#[test]
fn dirty_tree_blocks_planning_without_git_or_store_mutation() {
    let mut dirty = snapshot("feature/push", vec![remote("github", "widgets")]);
    dirty.dirty_paths = vec!["a.txt".into()];
    let git = fake_git(dirty, branch("feature/push", 1));
    let store = FakeStore::new();

    let plan_error = plan_push_changes(
        &git,
        &store,
        &FakePolicy { yaml: None },
        Path::new(ROOT),
        None,
    )
    .unwrap_err();
    assert_eq!(
        plan_error,
        AppError::DirtyWorkingTree {
            paths: vec!["a.txt".into()]
        }
    );
    assert!(git.executed.borrow().is_empty());
    assert!(store.projects.lock().unwrap().is_empty());
    assert!(store.operations.lock().unwrap().is_empty());
}

#[test]
fn clean_feature_branch_plans_fetch_and_non_force_push_with_upstream() {
    let git = fake_git(
        snapshot("feature/push", vec![remote("github", "widgets")]),
        branch("feature/push", 2),
    );
    let store = FakeStore::new();

    let (plan, planned_snapshot) = plan_push_changes(
        &git,
        &store,
        &FakePolicy { yaml: None },
        Path::new(ROOT),
        None,
    )
    .unwrap();

    assert!(matches!(
        plan.commands.as_slice(),
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
            && local_ref == "feature/push"
            && remote_ref == "feature/push"
    ));
    assert_eq!(planned_snapshot.selected_remote.as_deref(), Some("github"));
    assert!(store.projects.lock().unwrap().is_empty());
    assert!(store.operations.lock().unwrap().is_empty());
}

#[test]
fn branch_with_no_ahead_commits_is_noop_without_journal_row() {
    let git = fake_git(
        snapshot("feature/push", vec![remote("github", "widgets")]),
        branch("feature/push", 0),
    );
    let store = FakeStore::new();

    let (plan, planned_snapshot) = plan_push_changes(
        &git,
        &store,
        &FakePolicy { yaml: None },
        Path::new(ROOT),
        None,
    )
    .unwrap();
    let outcome = execute_push(
        &git,
        &store,
        &FakeClock(NOW.into()),
        &FakeIds::new(),
        &plan,
        &planned_snapshot,
    )
    .unwrap();

    assert!(plan.commands.is_empty());
    assert!(plan.summary.contains("Nothing to push"));
    assert_eq!(outcome.operation_id, "");
    assert_eq!(outcome.status, "noop");
    assert!(outcome.changed.is_empty());
    assert!(git.executed.borrow().is_empty());
    assert_eq!(store.projects.lock().unwrap().len(), 1);
    assert!(store.operations.lock().unwrap().is_empty());
}

#[test]
fn default_branch_push_is_rejected_as_protected_branch_misuse() {
    let git = fake_git(
        snapshot("main", vec![remote("github", "widgets")]),
        branch("main", 1),
    );
    let store = FakeStore::new();

    let error = plan_push_changes(
        &git,
        &store,
        &FakePolicy { yaml: None },
        Path::new(ROOT),
        None,
    )
    .unwrap_err();

    assert_eq!(
        error,
        AppError::Domain(WorkbenchError::ProtectedBranchMisuse {
            branch: "main".into()
        })
    );
    assert!(git.executed.borrow().is_empty());
    assert!(store.operations.lock().unwrap().is_empty());
}

#[test]
fn multiple_unmapped_remotes_are_not_resolved() {
    let git = fake_git(
        snapshot(
            "feature/push",
            vec![
                remote("origin", "widgets-mirror"),
                remote("github", "widgets"),
            ],
        ),
        branch("feature/push", 1),
    );
    let store = FakeStore::new();

    let error = plan_push_changes(
        &git,
        &store,
        &FakePolicy { yaml: None },
        Path::new(ROOT),
        None,
    )
    .unwrap_err();

    assert_eq!(
        error,
        AppError::RemoteNotResolved {
            candidates: vec!["origin".into(), "github".into()]
        }
    );
    assert!(git.executed.borrow().is_empty());
    assert!(store.operations.lock().unwrap().is_empty());
}

#[test]
fn successful_push_is_returned_by_project_operations_with_succeeded_steps() {
    let git = fake_git(
        snapshot("feature/push", vec![remote("github", "widgets")]),
        branch("feature/push", 2),
    );
    let store = FakeStore::new();
    let (confirmed_plan, planned_snapshot) = plan_push_changes(
        &git,
        &store,
        &FakePolicy { yaml: None },
        Path::new(ROOT),
        None,
    )
    .unwrap();

    let outcome = execute_push(
        &git,
        &store,
        &FakeClock(NOW.into()),
        &FakeIds::new(),
        &confirmed_plan,
        &planned_snapshot,
    )
    .unwrap();
    let operations = list_project_operations(&git, &store, Path::new(ROOT), None).unwrap();

    assert_eq!(outcome.status, "succeeded");
    assert_eq!(operations.len(), 1);
    assert_eq!(operations[0].kind, "push");
    assert_eq!(operations[0].status, "succeeded");
    assert_eq!(
        operations[0].plan_json,
        serde_json::to_string(&confirmed_plan).unwrap()
    );
    assert_eq!(operations[0].steps.len(), 2);
    assert!(operations[0]
        .steps
        .iter()
        .all(|step| step.status == StepStatus::Succeeded));
}

#[test]
fn listing_operations_without_stored_project_is_not_mapped() {
    let git = fake_git(
        snapshot("feature/push", vec![remote("github", "widgets")]),
        branch("feature/push", 1),
    );

    let error =
        list_project_operations(&git, &FakeStore::new(), Path::new(ROOT), None).unwrap_err();

    assert_eq!(error, AppError::RepositoryNotMapped);
}

#[test]
fn detached_head_is_rejected_before_remote_resolution_or_mutation() {
    let mut detached = snapshot("feature/push", vec![remote("github", "widgets")]);
    detached.detached_head = true;
    detached.branch = None;
    let git = fake_git(detached, branch("feature/push", 1));
    let store = FakeStore::new();

    let error = plan_push_changes(
        &git,
        &store,
        &FakePolicy { yaml: None },
        Path::new(ROOT),
        None,
    )
    .unwrap_err();

    assert_eq!(
        error,
        AppError::Usage {
            message: "detached HEAD cannot be pushed by gww".into()
        }
    );
    assert!(git.executed.borrow().is_empty());
    assert!(store.projects.lock().unwrap().is_empty());
    assert!(store.operations.lock().unwrap().is_empty());
}

#[test]
fn listing_operations_defaults_to_twenty_rows() {
    let git = fake_git(
        snapshot("feature/push", vec![remote("github", "widgets")]),
        branch("feature/push", 1),
    );
    let store = FakeStore::new();
    let ids = FakeIds::new();
    let (confirmed_plan, planned_snapshot) = plan_push_changes(
        &git,
        &store,
        &FakePolicy { yaml: None },
        Path::new(ROOT),
        None,
    )
    .unwrap();

    for _ in 0..21 {
        execute_push(
            &git,
            &store,
            &FakeClock(NOW.into()),
            &ids,
            &confirmed_plan,
            &planned_snapshot,
        )
        .unwrap();
    }

    let operations = list_project_operations(&git, &store, Path::new(ROOT), None).unwrap();
    assert_eq!(operations.len(), 20);
}
