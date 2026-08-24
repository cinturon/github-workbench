use std::cell::RefCell;
use std::path::PathBuf;
use workbench_application::fakes::{FakeClock, FakeGit, FakeIds, FakePolicy, FakeStore};
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

fn snap(
    branch: &str,
    dirty: Vec<String>,
    remotes: Vec<Remote>,
    detached: bool,
) -> RepositorySnapshot {
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
    let git = git(
        snap("main", vec![], vec![github_remote()], false),
        branch("main", 0, vec![]),
    );
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
fn open_records_local_project_without_remotes() {
    let git = git(
        snap("main", vec![], vec![], false),
        branch("main", 0, vec![]),
    );
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

    assert_eq!(out.snapshot.selected_remote, None);
    assert_eq!(out.project.remote_name, None);
    assert_eq!(out.project.github_host, None);
    assert_eq!(out.project.owner, None);
    assert_eq!(out.project.repo, None);
    assert_eq!(store.projects.lock().unwrap().len(), 1);
}

#[test]
fn invalid_policy_does_not_write_sqlite() {
    let git = git(
        snap("main", vec![], vec![github_remote()], false),
        branch("main", 0, vec![]),
    );
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
        Remote {
            name: "other".into(),
            url: "git@github.com:acme/other.git".into(),
        },
    ];
    let git = git(
        snap("main", vec![], remotes, false),
        branch("main", 0, vec![]),
    );
    let store = FakeStore::new();
    let err = open_repository(
        &git,
        &store,
        &FakePolicy { yaml: None },
        &FakeClock("2026-08-24T00:00:00Z".into()),
        &FakeIds::new(),
        PathBuf::from("/tmp/repo").as_path(),
        None,
    )
    .unwrap_err();
    assert!(matches!(err, AppError::RemoteNotResolved { .. }));
    assert!(store.projects.lock().unwrap().is_empty());
}

#[test]
fn status_recommends_start_issue_on_clean_main() {
    let git = git(
        snap("main", vec![], vec![github_remote()], false),
        branch("main", 0, vec![]),
    );
    let store = FakeStore::new();
    let out = repository_status(
        &git,
        &FakePolicy { yaml: None },
        PathBuf::from("/tmp/repo").as_path(),
        None,
        None,
    )
    .unwrap();
    assert!(out.recommended_next_action.contains("gww issue start"));
    assert!(store.projects.lock().unwrap().is_empty());
}

#[test]
fn status_recommends_push_when_ahead() {
    let git = git(
        snap(
            "feature/42-add-resumable-uploads",
            vec![],
            vec![github_remote()],
            false,
        ),
        branch("feature/42-add-resumable-uploads", 2, vec![]),
    );
    let out = repository_status(
        &git,
        &FakePolicy { yaml: None },
        PathBuf::from("/tmp/repo").as_path(),
        None,
        None,
    )
    .unwrap();
    assert!(out.recommended_next_action.contains("gww push --plan"));
}

#[test]
fn status_recommends_commit_when_dirty_feature_branch() {
    let git = git(
        snap(
            "feature/42-add-resumable-uploads",
            vec!["a.txt".into()],
            vec![github_remote()],
            false,
        ),
        branch("feature/42-add-resumable-uploads", 0, vec!["a.txt".into()]),
    );
    let out = repository_status(
        &git,
        &FakePolicy { yaml: None },
        PathBuf::from("/tmp/repo").as_path(),
        None,
        None,
    )
    .unwrap();
    assert!(out.recommended_next_action.contains("Commit your changes"));
}

#[test]
fn status_recommends_checkout_when_detached() {
    let git = git(
        snap("HEAD", vec![], vec![github_remote()], true),
        branch("HEAD", 0, vec![]),
    );
    let out = repository_status(
        &git,
        &FakePolicy { yaml: None },
        PathBuf::from("/tmp/repo").as_path(),
        None,
        None,
    )
    .unwrap();
    assert!(out.recommended_next_action.contains("Check out a branch"));
}
